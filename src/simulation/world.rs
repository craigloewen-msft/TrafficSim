//! Main simulation world that ties everything together
//!
//! This is the entry point for running the traffic simulation
//! without any Bevy dependencies.

use anyhow::{Context, Result};
use log::warn;
use ordered_float::OrderedFloat;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

use super::building::{SimFactory, SimApartment, SimShop};
use super::car::{CarUpdateResult, SimCar};
use super::game_state::{GameState, COST_FACTORY, COST_APARTMENT, COST_ROAD, COST_SHOP};
use super::intersection::SimIntersection;
use super::road_network::SimRoadNetwork;
use super::types::{
    CarId, FactoryId, ApartmentId, IntersectionId, Position, RoadId, ShopId, SimId, SimRoad, TripType,
    VehicleType,
};

/// Global demand metrics for the simulation
///
/// Tracks building busy states - i.e., which buildings currently have their
/// vehicle out on the road.
#[derive(Debug, Clone, Default)]
pub struct GlobalDemand {
    /// Number of factories with trucks out (busy making deliveries)
    pub factories_waiting: usize,
    /// Total number of factories
    pub total_factories: usize,
    /// Number of shops (always 0 - shops are passive)
    pub shops_waiting: usize,
    /// Total number of shops
    pub total_shops: usize,
    /// Number of apartments with cars out (busy)
    pub apartments_waiting: usize,
    /// Total number of apartments
    pub total_apartments: usize,
}

/// The main simulation world
pub struct SimWorld {
    /// Road network for pathfinding
    pub road_network: SimRoadNetwork,

    /// All intersections
    pub intersections: HashMap<IntersectionId, SimIntersection>,

    /// All cars
    pub cars: HashMap<CarId, SimCar>,

    /// All apartments
    pub apartments: HashMap<ApartmentId, SimApartment>,

    /// All factories
    pub factories: HashMap<FactoryId, SimFactory>,

    /// All shops
    pub shops: HashMap<ShopId, SimShop>,

    /// Next ID to assign
    next_id: usize,

    /// Simulation time
    pub time: f32,

    /// Optional seeded RNG for reproducible simulations
    rng: Option<StdRng>,

    /// Game state tracking (optional - only used when playing as a game)
    pub game_state: Option<GameState>,
}

impl Default for SimWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl SimWorld {
    fn new_internal(rng: Option<StdRng>, game_state: Option<GameState>) -> Self {
        Self {
            road_network: SimRoadNetwork::new(),
            intersections: HashMap::new(),
            cars: HashMap::new(),
            apartments: HashMap::new(),
            factories: HashMap::new(),
            shops: HashMap::new(),
            next_id: 0,
            time: 0.0,
            rng,
            game_state,
        }
    }

    pub fn new() -> Self {
        Self::new_internal(None, None)
    }

    /// Create a new SimWorld with a seeded RNG for reproducible simulations
    pub fn new_with_seed(seed: u64) -> Self {
        Self::new_internal(Some(StdRng::seed_from_u64(seed)), None)
    }

    /// Create a new SimWorld with game state enabled (for playing as a game)
    pub fn new_with_game() -> Self {
        Self::new_internal(None, Some(GameState::new()))
    }

    /// Get a random value in the given range, using seeded RNG if available
    fn random_range(&mut self, range: std::ops::Range<f32>) -> f32 {
        match &mut self.rng {
            Some(rng) => rng.random_range(range),
            None => rand::rng().random_range(range),
        }
    }

    /// Choose a random element from a slice, using seeded RNG if available
    fn choose_random<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            return None;
        }
        match &mut self.rng {
            Some(rng) => slice.choose(rng),
            None => slice.choose(&mut rand::rng()),
        }
    }

    fn next_sim_id(&mut self) -> SimId {
        let id = SimId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Attempts to charge the given cost from the game state if one exists.
    /// Returns `true` when no game state is attached so headless simulations
    /// can operate without budget constraints.
    fn spend_for_game(&mut self, cost: i32) -> bool {
        match &mut self.game_state {
            Some(game_state) => game_state.spend(cost),
            None => true,
        }
    }

    /// Add an intersection to the world
    pub fn add_intersection(&mut self, position: Position) -> IntersectionId {
        let id = IntersectionId(self.next_sim_id());
        let intersection = SimIntersection::new(id, position);
        self.intersections.insert(id, intersection);
        self.road_network.add_intersection(id, position);
        id
    }

    /// Add a road between two intersections (one-way)
    pub fn add_road(
        &mut self,
        start: IntersectionId,
        end: IntersectionId,
        is_two_way: bool,
    ) -> Result<RoadId> {
        let start_pos = *self
            .road_network
            .get_intersection_position(start)
            .context("Start intersection not found")?;

        let end_pos = *self
            .road_network
            .get_intersection_position(end)
            .context("End intersection not found")?;

        let id = RoadId(self.next_sim_id());
        let road = SimRoad::new(id, start, end, &start_pos, &end_pos, is_two_way);
        self.road_network.add_road(road);
        Ok(id)
    }

    /// Add a two-way road between intersections (creates two logical roads)
    pub fn add_two_way_road(
        &mut self,
        start: IntersectionId,
        end: IntersectionId,
    ) -> Result<(RoadId, RoadId)> {
        let forward = self.add_road(start, end, true)?;
        let backward = self.add_road(end, start, true)?;
        Ok((forward, backward))
    }

    /// Add an apartment at an intersection
    pub fn add_apartment(&mut self, intersection_id: IntersectionId) -> ApartmentId {
        let id = ApartmentId(self.next_sim_id());
        let apartment = SimApartment::new(id, intersection_id);
        self.apartments.insert(id, apartment);
        id
    }

    /// Add a factory at an intersection
    pub fn add_factory(&mut self, intersection_id: IntersectionId) -> FactoryId {
        let id = FactoryId(self.next_sim_id());
        let factory = SimFactory::new(id, intersection_id);
        self.factories.insert(id, factory);
        id
    }

    /// Add a shop at an intersection
    pub fn add_shop(&mut self, intersection_id: IntersectionId) -> ShopId {
        let id = ShopId(self.next_sim_id());
        let shop = SimShop::new(id, intersection_id);
        self.shops.insert(id, shop);
        id
    }

    /// Add an apartment with game cost checking
    /// Returns Some(apartment_id) if successful, None if insufficient funds
    pub fn try_add_apartment(&mut self, intersection_id: IntersectionId) -> Option<ApartmentId> {
        if !self.spend_for_game(COST_APARTMENT) {
            return None;
        }
        Some(self.add_apartment(intersection_id))
    }

    /// Add a factory with game cost checking
    /// Returns Some(factory_id) if successful, None if insufficient funds
    pub fn try_add_factory(&mut self, intersection_id: IntersectionId) -> Option<FactoryId> {
        if !self.spend_for_game(COST_FACTORY) {
            return None;
        }
        Some(self.add_factory(intersection_id))
    }

    /// Add a shop with game cost checking
    /// Returns Some(shop_id) if successful, None if insufficient funds
    pub fn try_add_shop(&mut self, intersection_id: IntersectionId) -> Option<ShopId> {
        if !self.spend_for_game(COST_SHOP) {
            return None;
        }
        Some(self.add_shop(intersection_id))
    }

    /// Add a two-way road with game cost checking
    /// Returns Some((forward, backward)) if successful, None if insufficient funds
    pub fn try_add_two_way_road(
        &mut self,
        start: IntersectionId,
        end: IntersectionId,
    ) -> Result<Option<(RoadId, RoadId)>> {
        if !self.spend_for_game(COST_ROAD) {
            return Ok(None);
        }
        self.add_two_way_road(start, end).map(Some)
    }

    /// Add roads at positions with game cost checking
    /// Returns Some(...) if successful, None if insufficient funds
    pub fn try_add_road_at_positions(
        &mut self,
        start_pos: Position,
        end_pos: Position,
        snap_distance: f32,
    ) -> Result<Option<(IntersectionId, IntersectionId, RoadId, RoadId)>> {
        if !self.spend_for_game(COST_ROAD) {
            return Ok(None);
        }
        self.add_road_at_positions(start_pos, end_pos, snap_distance)
            .map(Some)
    }

    /// Remove an apartment from the world
    /// Returns the cars that were associated with the apartment (if any)
    pub fn remove_apartment(&mut self, apartment_id: ApartmentId) -> Vec<CarId> {
        let apartment = match self.apartments.remove(&apartment_id) {
            Some(a) => a,
            None => return Vec::new(),
        };
        apartment.cars.into_iter().flatten().collect()
    }

    /// Remove a factory from the world
    pub fn remove_factory(&mut self, factory_id: FactoryId) {
        self.factories.remove(&factory_id);
    }

    /// Remove a shop from the world
    pub fn remove_shop(&mut self, shop_id: ShopId) {
        self.shops.remove(&shop_id);
    }

    /// Remove a road from the world
    /// Cars on the road will be despawned
    pub fn remove_road(&mut self, road_id: RoadId) -> Result<()> {
        let cars_on_road = self.road_network.remove_road(road_id)?;

        // Despawn all cars that were on the removed road
        for car_id in cars_on_road {
            self.despawn_car(car_id);
        }

        Ok(())
    }

    /// Remove an intersection and all connected roads
    /// Cars on affected roads will be despawned
    /// Buildings at the intersection will be removed
    pub fn remove_intersection(&mut self, intersection_id: IntersectionId) -> Result<()> {
        // Remove any buildings at this intersection
        let apartments_to_remove: Vec<ApartmentId> = self
            .apartments
            .iter()
            .filter(|(_, a)| a.intersection_id == intersection_id)
            .map(|(id, _)| *id)
            .collect();

        for apartment_id in apartments_to_remove {
            self.remove_apartment(apartment_id);
        }

        let factories_to_remove: Vec<FactoryId> = self
            .factories
            .iter()
            .filter(|(_, f)| f.intersection_id == intersection_id)
            .map(|(id, _)| *id)
            .collect();

        for factory_id in factories_to_remove {
            self.remove_factory(factory_id);
        }

        let shops_to_remove: Vec<ShopId> = self
            .shops
            .iter()
            .filter(|(_, s)| s.intersection_id == intersection_id)
            .map(|(id, _)| *id)
            .collect();

        for shop_id in shops_to_remove {
            self.remove_shop(shop_id);
        }

        // Remove the intersection from intersections collection
        self.intersections.remove(&intersection_id);

        // Remove intersection and roads from road network
        let (_, cars_on_roads) = self.road_network.remove_intersection(intersection_id)?;

        // Despawn all cars that were on removed roads
        for car_id in cars_on_roads {
            self.despawn_car(car_id);
        }

        // Recalculate paths for remaining cars that might have been using deleted roads
        self.recalculate_car_paths();

        Ok(())
    }

    /// Remove a two-way road (both directions)
    /// Cars on either direction will be despawned
    pub fn remove_two_way_road(
        &mut self,
        intersection_a: IntersectionId,
        intersection_b: IntersectionId,
    ) -> Result<()> {
        // Find and remove both directions of the road
        if let Ok(forward_road) = self
            .road_network
            .find_road_between(intersection_a, intersection_b)
        {
            self.remove_road(forward_road)?;
        }

        if let Ok(backward_road) = self
            .road_network
            .find_road_between(intersection_b, intersection_a)
        {
            self.remove_road(backward_road)?;
        }

        Ok(())
    }

    /// Despawn a car and clean up references
    fn despawn_car(&mut self, car_id: CarId) {
        // Get car info before removing
        let car_info = self
            .cars
            .get(&car_id)
            .map(|c| (c.origin_apartment, c.origin_factory));

        self.cars.remove(&car_id);
        self.road_network.remove_car_from_tracking(car_id);

        if let Some((origin_apartment, origin_factory)) = car_info {
            // Clear apartment car reference
            if let Some(apartment_id) = origin_apartment {
                if let Some(apartment) = self.apartments.get_mut(&apartment_id) {
                    for car_slot in &mut apartment.cars {
                        if *car_slot == Some(car_id) {
                            *car_slot = None;
                            break;
                        }
                    }
                }
            }

            // Clear factory truck reference
            if let Some(factory_id) = origin_factory {
                if let Some(factory) = self.factories.get_mut(&factory_id) {
                    if factory.truck == Some(car_id) {
                        factory.truck = None;
                    }
                }
            }
        }
    }

    /// Recalculate paths for all cars that might have invalid paths
    fn recalculate_car_paths(&mut self) {
        let car_ids: Vec<CarId> = self.cars.keys().copied().collect();

        for car_id in car_ids {
            if let Some(car) = self.cars.get(&car_id) {
                // Get the car's final destination
                let destination = match car.path.last() {
                    Some(dest) => *dest,
                    None => continue, // No path to recalculate
                };

                // Get the current intersection the car is heading to
                let current_target = match car.path.first() {
                    Some(target) => *target,
                    None => continue,
                };

                // Try to find a new path from current target to destination
                let new_path = self.road_network.find_path(current_target, destination);

                match new_path {
                    Some(path) => {
                        // Update the car's path
                        if let Some(car) = self.cars.get_mut(&car_id) {
                            car.path = std::iter::once(current_target).chain(path).collect();
                        }
                    }
                    None => {
                        // No valid path exists - despawn the car
                        self.despawn_car(car_id);
                    }
                }
            }
        }
    }

    /// Split a road at a given position to create a new intersection
    /// Returns the new intersection ID and the IDs of the new roads
    pub fn split_road_at_position(
        &mut self,
        road_id: RoadId,
        split_position: Position,
    ) -> Result<(IntersectionId, RoadId, RoadId)> {
        let road = self
            .road_network
            .get_road(road_id)
            .context("Road not found")?
            .clone();

        let start_intersection = road.start_intersection;
        let end_intersection = road.end_intersection;
        let is_two_way = road.is_two_way;

        // Get cars that were on this road before removing it
        let cars_on_road = self.road_network.get_cars_on_road(road_id);

        // Remove the original road
        self.road_network.remove_road(road_id)?;

        // Create new intersection at split position
        let new_intersection = self.add_intersection(split_position);

        // Create new roads
        let first_road = self.add_road(start_intersection, new_intersection, is_two_way)?;
        let second_road = self.add_road(new_intersection, end_intersection, is_two_way)?;

        // If two-way, also create reverse roads
        if is_two_way {
            // Remove the reverse road if it exists
            if let Ok(reverse_road) = self
                .road_network
                .find_road_between(end_intersection, start_intersection)
            {
                self.road_network.remove_road(reverse_road)?;
            }

            self.add_road(new_intersection, start_intersection, is_two_way)?;
            self.add_road(end_intersection, new_intersection, is_two_way)?;
        }

        // Despawn cars that were on the split road (they need to recalculate)
        for car_id in cars_on_road {
            self.despawn_car(car_id);
        }

        Ok((new_intersection, first_road, second_road))
    }

    /// Dynamically add a two-way road between two positions
    /// If positions are close to existing intersections, reuse them
    /// If a position is close to an existing road, split that road
    pub fn add_road_at_positions(
        &mut self,
        start_pos: Position,
        end_pos: Position,
        snap_distance: f32,
    ) -> Result<(IntersectionId, IntersectionId, RoadId, RoadId)> {
        // Find or create start intersection
        let start_intersection = self.find_or_create_intersection(start_pos, snap_distance)?;

        // Find or create end intersection
        let end_intersection = self.find_or_create_intersection(end_pos, snap_distance)?;

        // Check if these intersections are already connected
        if self
            .road_network
            .find_road_between(start_intersection, end_intersection)
            .is_ok()
        {
            anyhow::bail!("Road already exists between these intersections");
        }

        // Create the two-way road
        let (forward, backward) = self.add_two_way_road(start_intersection, end_intersection)?;

        Ok((start_intersection, end_intersection, forward, backward))
    }

    /// Find an existing intersection near a position, or create a new one
    /// If the position is near an existing road, split that road
    fn find_or_create_intersection(
        &mut self,
        position: Position,
        snap_distance: f32,
    ) -> Result<IntersectionId> {
        // First, check if there's an existing intersection nearby
        if let Some(closest_intersection) = self.road_network.find_closest_intersection(&position) {
            if let Some(intersection_pos) = self
                .road_network
                .get_intersection_position(closest_intersection)
            {
                if position.distance(intersection_pos) <= snap_distance {
                    return Ok(closest_intersection);
                }
            }
        }

        // Check if position is close to an existing road (for splitting)
        if let Some((road_id, closest_point, _, _)) =
            self.road_network.find_closest_point_on_road(&position)
        {
            if position.distance(&closest_point) <= snap_distance {
                // Split the road at this position
                let (new_intersection, _, _) =
                    self.split_road_at_position(road_id, closest_point)?;
                return Ok(new_intersection);
            }
        }

        // No nearby intersection or road - create new intersection
        Ok(self.add_intersection(position))
    }

    /// Spawn a vehicle from a given intersection to a destination
    pub fn spawn_vehicle(
        &mut self,
        from_intersection: IntersectionId,
        to_intersection: IntersectionId,
        vehicle_type: VehicleType,
        trip_type: TripType,
        origin_apartment: Option<ApartmentId>,
        origin_factory: Option<FactoryId>,
    ) -> Result<CarId> {
        // Find connected roads from the starting intersection
        let connected_roads = self
            .road_network
            .get_connected_roads(from_intersection)
            .context("Starting intersection not found in road network")?;

        if connected_roads.is_empty() {
            anyhow::bail!("No roads connected to starting intersection");
        }

        // Find the path
        let path = self
            .road_network
            .find_path(from_intersection, to_intersection)
            .context("No path found to destination")?;

        if path.is_empty() && from_intersection != to_intersection {
            anyhow::bail!("Empty path but different start/end");
        }

        // Get the first road in the path
        let first_target = path.first().copied().unwrap_or(to_intersection);
        let road_id = self
            .road_network
            .find_road_between(from_intersection, first_target)
            .context("No road to first path intersection")?;

        let road = self
            .road_network
            .get_road(road_id)
            .context("Road not found")?;

        let road_angle = road.angle;

        let start_pos = *self
            .road_network
            .get_intersection_position(from_intersection)
            .context("Start intersection position not found")?;

        // Generate random speed (trucks are faster)
        let speed = match vehicle_type {
            VehicleType::Car => self.random_range(2.0..6.0),
            VehicleType::Truck => self.random_range(4.0..8.0),
        };

        let id = CarId(self.next_sim_id());
        let car = SimCar::new(
            id,
            speed,
            road_id,
            from_intersection,
            path,
            start_pos,
            road_angle,
            vehicle_type,
            trip_type,
            origin_apartment,
            origin_factory,
        );

        // Register car on road
        self.road_network.update_car_road_position(
            id,
            road_id,
            OrderedFloat(0.0),
            false,
            None,
            OrderedFloat(0.0),
        )?;

        self.cars.insert(id, car);
        Ok(id)
    }

    /// Update all cars in the simulation
    fn update_cars(&mut self, delta_secs: f32) -> Vec<(CarId, CarUpdateResult)> {
        let mut results = Vec::new();

        // Collect car IDs to avoid borrow issues
        let car_ids: Vec<CarId> = self.cars.keys().copied().collect();

        for car_id in car_ids {
            // Get car mutably, update it, then process result
            if let Some(mut car) = self.cars.remove(&car_id) {
                let result =
                    car.update(delta_secs, &mut self.road_network, &mut self.intersections);

                match result {
                    Ok(CarUpdateResult::Continue) => {
                        self.cars.insert(car_id, car);
                    }
                    Ok(CarUpdateResult::Despawn) => {
                        // Put car back temporarily so tick() can read its info
                        self.cars.insert(car_id, car);
                        results.push((car_id, CarUpdateResult::Despawn));
                    }
                    Ok(CarUpdateResult::ArrivedAtDestination(dest)) => {
                        // Put car back temporarily so tick() can read its info
                        self.cars.insert(car_id, car);
                        results.push((car_id, CarUpdateResult::ArrivedAtDestination(dest)));
                    }
                    Err(_) => {
                        // Put car back temporarily so tick() can read its info
                        self.cars.insert(car_id, car);
                        results.push((car_id, CarUpdateResult::Despawn));
                    }
                }
            }
        }

        results
    }

    /// Update all intersections
    fn update_intersections(&mut self, delta_secs: f32) {
        for intersection in self.intersections.values_mut() {
            intersection.update_timer(delta_secs);
        }
    }

    /// Update all shops
    fn update_shops(&mut self, _delta_secs: f32) {
        // Shops no longer have demand that increases over time
    }

    /// Update all factories
    /// Returns (workers_done_apartment_ids, trucks_to_dispatch)
    fn update_factories(
        &mut self,
        delta_secs: f32,
    ) -> (Vec<(FactoryId, ApartmentId)>, Vec<(FactoryId, IntersectionId)>) {
        let mut workers_done = Vec::new();
        let mut trucks_to_dispatch = Vec::new();

        // Get all shops - trucks always dispatch if deliveries are ready
        let shop_intersections: Vec<IntersectionId> =
            self.shops.values().map(|s| s.intersection_id).collect();

        // Collect factory IDs to avoid borrow issues
        let factory_ids: Vec<FactoryId> = self.factories.keys().copied().collect();

        for factory_id in factory_ids {
            let factory = match self.factories.get_mut(&factory_id) {
                Some(f) => f,
                None => continue,
            };

            // Update factory and get apartment_ids of workers who finished their shift
            let finished_apartment_ids = factory.update(delta_secs);

            // Record which apartments have workers done
            for apartment_id in finished_apartment_ids {
                workers_done.push((factory_id, apartment_id));
            }

            // If truck is available and there are deliveries ready and shops exist
            if factory.truck_available()
                && factory.deliveries_ready > 0
                && !shop_intersections.is_empty()
            {
                // Take a delivery for dispatch
                if factory.take_delivery() {
                    // Pick a random shop (use index based on factory id for determinism)
                    let shop_index = factory_id.0 .0 % shop_intersections.len();
                    let shop_intersection = shop_intersections[shop_index];
                    trucks_to_dispatch.push((factory_id, shop_intersection));
                }
            }
        }

        (workers_done, trucks_to_dispatch)
    }

    /// Spawn workers from apartments to factories
    fn spawn_workers(&mut self) {
        // Get all factories that can accept workers (truck is home)
        let factories_accepting: Vec<(FactoryId, IntersectionId)> = self
            .factories
            .values()
            .filter(|f| f.can_accept_workers())
            .map(|f| (f.id, f.intersection_id))
            .collect();

        if factories_accepting.is_empty() {
            return;
        }

        // Collect apartment IDs and available car slots
        let mut apartment_slots_to_spawn = Vec::new();
        
        for (apartment_id, apartment) in &self.apartments {
            let apartment_intersection = apartment.intersection_id;
            
            // Try to spawn a car from each empty slot (up to 10 cars per apartment)
            for (slot_index, car_slot) in apartment.cars.iter().enumerate() {
                // Only spawn if this slot doesn't have a car out
                if car_slot.is_none() {
                    apartment_slots_to_spawn.push((*apartment_id, slot_index, apartment_intersection));
                }
            }
        }

        // Now spawn cars for each available slot
        for (apartment_id, slot_index, apartment_intersection) in apartment_slots_to_spawn {
            // Choose random factory
            let (_factory_id, factory_intersection) = match self.choose_random(&factories_accepting)
            {
                Some(&(fid, fi)) => (fid, fi),
                None => continue,
            };

            // Spawn car going to work
            match self.spawn_vehicle(
                apartment_intersection,
                factory_intersection,
                VehicleType::Car,
                TripType::Outbound,
                Some(apartment_id),
                None,
            ) {
                Ok(car_id) => {
                    if let Some(apartment) = self.apartments.get_mut(&apartment_id) {
                        apartment.cars[slot_index] = Some(car_id);
                    }
                }
                Err(_) => continue,
            }
        }
    }

    /// Main simulation tick
    pub fn tick(&mut self, delta_secs: f32) {
        self.time += delta_secs;

        // Update game state if enabled
        if let Some(game_state) = &mut self.game_state {
            game_state.update(delta_secs);
        }

        // Update intersections
        self.update_intersections(delta_secs);

        // Update shops
        self.update_shops(delta_secs);

        // Update factories - get workers done and trucks to dispatch
        let (workers_done, trucks_to_dispatch) = self.update_factories(delta_secs);

        // Send workers home after their shift
        for (factory_id, apartment_id) in workers_done {
            // Get the apartment intersection
            let apartment_intersection = match self.apartments.get(&apartment_id) {
                Some(a) => a.intersection_id,
                None => continue,
            };

            // Get the factory intersection
            let factory_intersection = match self.factories.get(&factory_id) {
                Some(f) => f.intersection_id,
                None => continue,
            };

            // Spawn car returning home
            let _ = self.spawn_vehicle(
                factory_intersection,
                apartment_intersection,
                VehicleType::Car,
                TripType::Return,
                Some(apartment_id),
                Some(factory_id),
            );
        }

        // Dispatch trucks to make deliveries
        for (factory_id, shop_intersection) in trucks_to_dispatch {
            let factory_intersection = match self.factories.get(&factory_id) {
                Some(f) => f.intersection_id,
                None => continue,
            };

            // Spawn truck for delivery
            match self.spawn_vehicle(
                factory_intersection,
                shop_intersection,
                VehicleType::Truck,
                TripType::Outbound,
                None,
                Some(factory_id),
            ) {
                Ok(truck_id) => {
                    if let Some(factory) = self.factories.get_mut(&factory_id) {
                        factory.truck = Some(truck_id);
                    }
                }
                Err(_) => {
                    // Failed to spawn truck, return delivery to ready
                    if let Some(factory) = self.factories.get_mut(&factory_id) {
                        factory.deliveries_ready += 1;
                    }
                }
            }
        }

        // Spawn workers from apartments
        self.spawn_workers();

        // Update cars and process results
        let car_results = self.update_cars(delta_secs);

        // Process car arrivals
        for (car_id, result) in car_results {
            match result {
                CarUpdateResult::ArrivedAtDestination(dest) => {
                    // Get car info before processing
                    let car_info = self.cars.get(&car_id).map(|c| {
                        (
                            c.vehicle_type,
                            c.trip_type,
                            c.origin_apartment,
                            c.origin_factory,
                        )
                    });

                    if let Some((vehicle_type, trip_type, origin_apartment, origin_factory)) = car_info
                    {
                        match (vehicle_type, trip_type) {
                            (VehicleType::Car, TripType::Outbound) => {
                                // Worker arrived at factory - try to register them with their apartment_id
                                let mut worker_accepted = false;
                                let mut destination_factory: Option<FactoryId> = None;
                                if let Some(apartment_id) = origin_apartment {
                                    if let Some((factory_id, factory)) = self
                                        .factories
                                        .iter_mut()
                                        .find(|(_, f)| f.intersection_id == dest)
                                    {
                                        worker_accepted = factory.receive_worker(apartment_id);
                                        destination_factory = Some(*factory_id);
                                    }
                                }

                                if worker_accepted {
                                    // Remove car from tracking while at work (will respawn when returning home)
                                    self.road_network.remove_car_from_tracking(car_id);
                                    self.cars.remove(&car_id);
                                } else {
                                    // Factory rejected worker (truck out or full), send them back home
                                    if let Some(apartment_id) = origin_apartment {
                                        let apartment_intersection =
                                            self.apartments.get(&apartment_id).map(|a| a.intersection_id);
                                        if let Some(apartment_intersection) = apartment_intersection {
                                            // Spawn car returning home
                                            let _ = self.spawn_vehicle(
                                                dest,
                                                apartment_intersection,
                                                VehicleType::Car,
                                                TripType::Return,
                                                Some(apartment_id),
                                                destination_factory,
                                            );
                                        }
                                    }
                                    // Despawn the current car
                                    self.road_network.remove_car_from_tracking(car_id);
                                    self.cars.remove(&car_id);
                                }
                            }
                            (VehicleType::Car, TripType::Return) => {
                                let commute_distance = match (origin_apartment, origin_factory) {
                                    (Some(apartment_id), Some(factory_id)) => {
                                        let apartment_position = self
                                            .apartments
                                            .get(&apartment_id)
                                            .and_then(|apartment| {
                                                self.road_network.get_intersection_position(
                                                    apartment.intersection_id,
                                                )
                                            })
                                            .copied();
                                        let factory_position = self
                                            .factories
                                            .get(&factory_id)
                                            .and_then(|factory| {
                                                self.road_network.get_intersection_position(
                                                    factory.intersection_id,
                                                )
                                            })
                                            .copied();

                                        match (apartment_position, factory_position) {
                                            (Some(apartment_pos), Some(factory_pos)) => {
                                                apartment_pos.distance(&factory_pos)
                                            }
                                            _ => {
                                                warn!(
                                                    "Missing apartment or factory position for worker commute; defaulting to a zero-distance commute, which applies the maximum commute penalty"
                                                );
                                                0.0
                                            }
                                        }
                                    }
                                    _ => {
                                        warn!(
                                            "Missing worker identifiers for commute penalty; defaulting to a zero-distance commute, which applies the maximum commute penalty"
                                        );
                                        0.0
                                    }
                                };
                                // Worker returned home - clear car reference and despawn
                                if let Some(apartment_id) = origin_apartment {
                                    if let Some(apartment) = self.apartments.get_mut(&apartment_id) {
                                        // Find and clear the car slot
                                        for car_slot in &mut apartment.cars {
                                            if *car_slot == Some(car_id) {
                                                *car_slot = None;
                                                break;
                                            }
                                        }
                                    }
                                }
                                // Track worker trip completion in game state
                                if let Some(game_state) = &mut self.game_state {
                                    game_state.complete_worker_trip(commute_distance);
                                }
                                self.road_network.remove_car_from_tracking(car_id);
                                self.cars.remove(&car_id);
                            }
                            (VehicleType::Truck, TripType::Outbound) => {
                                // Truck delivered to shop
                                if let Some(shop) =
                                    self.shops.values_mut().find(|s| s.intersection_id == dest)
                                {
                                    shop.receive_delivery();
                                }
                                // Now spawn truck returning to factory
                                if let Some(factory_id) = origin_factory {
                                    let factory_intersection =
                                        self.factories.get(&factory_id).map(|f| f.intersection_id);
                                    if let Some(factory_intersection) = factory_intersection {
                                        // Spawn truck returning
                                        match self.spawn_vehicle(
                                            dest,
                                            factory_intersection,
                                            VehicleType::Truck,
                                            TripType::Return,
                                            None,
                                            Some(factory_id),
                                        ) {
                                            Ok(new_truck_id) => {
                                                if let Some(factory) =
                                                    self.factories.get_mut(&factory_id)
                                                {
                                                    factory.truck = Some(new_truck_id);
                                                }
                                            }
                                            Err(_) => {
                                                // Truck can't return, just clear reference
                                                if let Some(factory) =
                                                    self.factories.get_mut(&factory_id)
                                                {
                                                    factory.truck = None;
                                                }
                                            }
                                        }
                                    }
                                }
                                // Despawn old truck entity
                                self.road_network.remove_car_from_tracking(car_id);
                                self.cars.remove(&car_id);
                            }
                            (VehicleType::Truck, TripType::Return) => {
                                // Truck returned to factory - clear reference and despawn
                                if let Some(factory_id) = origin_factory {
                                    if let Some(factory) = self.factories.get_mut(&factory_id) {
                                        factory.truck = None;
                                    }
                                }
                                // Track shop delivery completion in game state
                                if let Some(game_state) = &mut self.game_state {
                                    game_state.complete_shop_delivery();
                                }
                                self.road_network.remove_car_from_tracking(car_id);
                                self.cars.remove(&car_id);
                            }
                        }
                    }
                }
                CarUpdateResult::Despawn => {
                    // Clean up references for unexpectedly despawned vehicles
                    if let Some(car) = self.cars.get(&car_id) {
                        if let Some(apartment_id) = car.origin_apartment {
                            if let Some(apartment) = self.apartments.get_mut(&apartment_id) {
                                // Find and clear the car slot
                                for car_slot in &mut apartment.cars {
                                    if *car_slot == Some(car_id) {
                                        *car_slot = None;
                                        break;
                                    }
                                }
                            }
                        }
                        if let Some(factory_id) = car.origin_factory {
                            if let Some(factory) = self.factories.get_mut(&factory_id) {
                                factory.truck = None;
                            }
                        }
                    }
                    self.road_network.remove_car_from_tracking(car_id);
                    self.cars.remove(&car_id);
                }
                CarUpdateResult::Continue => {}
            }
        }
    }

    /// Create a default test world with some roads and buildings
    pub fn create_test_world() -> Self {
        Self::build_test_world(SimWorld::new())
    }

    /// Create a default test world with a seeded RNG for reproducible simulations
    pub fn create_test_world_with_seed(seed: u64) -> Self {
        Self::build_test_world(SimWorld::new_with_seed(seed))
    }

    /// Internal helper to build the test world structure
    pub fn build_test_world(mut world: SimWorld) -> Self {
        // Create a 3x3 grid of intersections (main roads)
        let spacing = 20.0;
        let mut grid = [[IntersectionId(SimId(0)); 3]; 3];

        for row in 0..3 {
            for col in 0..3 {
                let x = (col as f32 - 1.0) * spacing;
                let z = (row as f32 - 1.0) * spacing;
                grid[row][col] = world.add_intersection(Position::new(x, 0.0, z));
            }
        }

        // Connect grid horizontally
        for row in 0..3 {
            for col in 0..2 {
                let _ = world.add_two_way_road(grid[row][col], grid[row][col + 1]);
            }
        }

        // Connect grid vertically
        for row in 0..2 {
            for col in 0..3 {
                let _ = world.add_two_way_road(grid[row][col], grid[row + 1][col]);
            }
        }

        // Add apartments (4 total) - offshoots from grid corners
        let apartment_data = vec![
            (grid[0][0], Position::new(-30.0, 0.0, -30.0)), // Top-left
            (grid[0][2], Position::new(30.0, 0.0, -30.0)),  // Top-right
            (grid[2][0], Position::new(-30.0, 0.0, 30.0)),  // Bottom-left
            (grid[2][2], Position::new(30.0, 0.0, 30.0)),   // Bottom-right
        ];

        for (grid_intersection, apartment_pos) in apartment_data {
            let apartment_intersection = world.add_intersection(apartment_pos);
            let _ = world.add_two_way_road(grid_intersection, apartment_intersection);
            world.add_apartment(apartment_intersection);
        }

        // Add factories (6 total) - offshoots from various grid points
        let factory_data = vec![
            (grid[0][1], Position::new(0.0, 0.0, -35.0)), // Top center
            (grid[1][0], Position::new(-35.0, 0.0, 0.0)), // Middle left
            (grid[1][2], Position::new(35.0, 0.0, 0.0)),  // Middle right
            (grid[2][1], Position::new(0.0, 0.0, 35.0)),  // Bottom center
            (grid[1][1], Position::new(-12.0, 0.0, -12.0)), // Center offset 1
            (grid[1][1], Position::new(12.0, 0.0, 12.0)), // Center offset 2
        ];

        for (grid_intersection, factory_pos) in factory_data {
            let factory_intersection = world.add_intersection(factory_pos);
            let _ = world.add_two_way_road(grid_intersection, factory_intersection);
            world.add_factory(factory_intersection);
        }

        // Add shops (2 total) - offshoots from grid
        let shop_data = vec![
            (grid[0][0], Position::new(-30.0, 0.0, -25.0)), // Near top-left
            (grid[2][2], Position::new(30.0, 0.0, 25.0)),   // Near bottom-right
        ];

        for (grid_intersection, shop_pos) in shop_data {
            let shop_intersection = world.add_intersection(shop_pos);
            let _ = world.add_two_way_road(grid_intersection, shop_intersection);
            world.add_shop(shop_intersection);
        }

        world
    }

    /// Print a summary of the world state
    pub fn print_summary(&self) {
        println!("=== Traffic Simulation Summary ===");
        println!("Time: {:.2}s", self.time);
        println!(
            "Intersections: {}, Roads: {}",
            self.road_network.intersection_count(),
            self.road_network.road_count()
        );
        println!("Cars: {}", self.cars.len());
        println!("Apartments: {}", self.apartments.len());
        println!("Factories: {}", self.factories.len());
        println!("Shops: {}", self.shops.len());
        println!();

        // Factory status
        println!("--- Factories ---");
        for factory in self.factories.values() {
            println!(
                "  Factory {:?}: deliveries={}/{}, workers={}, truck={}",
                factory.id.0,
                factory.deliveries_ready,
                factory.max_deliveries,
                factory.workers.len(),
                if factory.truck.is_some() {
                    "out"
                } else {
                    "home"
                }
            );
        }

        // Shop status
        println!("--- Shops ---");
        for shop in self.shops.values() {
            println!("  Shop {:?}: deliveries={}", shop.id.0, shop.cars_received);
        }

        // Active cars
        if !self.cars.is_empty() {
            println!("--- Active Cars ---");
            for car in self.cars.values() {
                println!(
                    "  Car {:?}: speed={:.1}, position=({:.1}, {:.1}), path_remaining={}",
                    car.id.0,
                    car.speed,
                    car.position.x,
                    car.position.z,
                    car.path.len()
                );
            }
        }

        // Global demand status
        let demand = self.calculate_global_demand();
        println!("--- Global Demand ---");
        println!(
            "  Factories waiting: {}/{}",
            demand.factories_waiting, demand.total_factories
        );
        println!(
            "  Shops waiting: {}/{}",
            demand.shops_waiting, demand.total_shops
        );
        println!(
            "  Apartments waiting: {}/{}",
            demand.apartments_waiting, demand.total_apartments
        );
    }

    /// Calculate global demand metrics
    ///
    /// Returns metrics showing building busy states:
    /// - Factories waiting: factories that can't accept workers (truck is out)
    /// - Shops waiting: always 0 (shops are passive receivers)
    /// - Apartments waiting: apartments with cars currently out (busy)
    pub fn calculate_global_demand(&self) -> GlobalDemand {
        let total_factories = self.factories.len();
        let total_shops = self.shops.len();
        let total_apartments = self.apartments.len();

        // Count factories that can accept workers (truck is home)
        let factories_accepting: usize = self
            .factories
            .values()
            .filter(|f| f.can_accept_workers())
            .count();

        // Count apartments with cars out (busy) - any car slot that is Some
        let apartments_busy: usize = self.apartments.values().filter(|a| a.cars.iter().any(|c| c.is_some())).count();

        // Simplified: factories waiting are those that can't accept workers (truck is out)
        let factories_waiting = total_factories - factories_accepting;

        // Simplified: shops always wait if they exist (no demand threshold)
        let shops_waiting = 0; // Shops are passive - they just receive deliveries

        // Apartments waiting are those with cars out (busy)
        let apartments_waiting = apartments_busy;

        GlobalDemand {
            factories_waiting,
            total_factories,
            shops_waiting,
            total_shops,
            apartments_waiting,
            total_apartments,
        }
    }

    /// Draw a visual map of the world in the terminal
    pub fn draw_map(&self) {
        // Find bounds of the world
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for pos in self.road_network.intersection_positions().values() {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_z = min_z.min(pos.z);
            max_z = max_z.max(pos.z);
        }

        // Add padding
        min_x -= 2.0;
        max_x += 2.0;
        min_z -= 2.0;
        max_z += 2.0;

        // Define grid size (characters per unit)
        let scale = 1.0; // 2 characters per world unit
        let width = ((max_x - min_x) * scale) as usize;
        let height = ((max_z - min_z) * scale) as usize;

        // Create grid
        let mut grid = vec![vec![' '; width]; height];

        // Helper to convert world coords to grid coords
        let to_grid = |x: f32, z: f32| -> (usize, usize) {
            let col = ((max_x - x) * scale) as usize;
            // Flip the Z-axis by subtracting from max_z instead of min_z
            let row = ((max_z - z) * scale) as usize;
            (row.min(height - 1), col.min(width - 1))
        };

        // Draw roads
        for road in self.road_network.roads().values() {
            let start_pos = self
                .road_network
                .get_intersection_position(road.start_intersection)
                .unwrap();
            let end_pos = self
                .road_network
                .get_intersection_position(road.end_intersection)
                .unwrap();

            let (start_row, start_col) = to_grid(start_pos.x, start_pos.z);
            let (end_row, end_col) = to_grid(end_pos.x, end_pos.z);

            // Simple line drawing (Bresenham-like)
            let dx = (end_col as i32 - start_col as i32).abs();
            let dy = (end_row as i32 - start_row as i32).abs();
            let sx = if start_col < end_col { 1 } else { -1 };
            let sy = if start_row < end_row { 1 } else { -1 };

            let mut err = dx - dy;
            let mut x = start_col as i32;
            let mut y = start_row as i32;

            loop {
                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    let ux = x as usize;
                    let uy = y as usize;
                    if grid[uy][ux] == ' ' {
                        grid[uy][ux] = '·';
                    }
                }

                if x == end_col as i32 && y == end_row as i32 {
                    break;
                }

                let e2 = 2 * err;
                if e2 > -dy {
                    err -= dy;
                    x += sx;
                }
                if e2 < dx {
                    err += dx;
                    y += sy;
                }
            }
        }

        // Draw intersections
        for (id, pos) in self.road_network.intersection_positions() {
            let (row, col) = to_grid(pos.x, pos.z);

            // Check what's at this intersection
            let has_apartment = self.apartments.values().any(|a| a.intersection_id == *id);
            let has_factory = self.factories.values().any(|f| f.intersection_id == *id);
            let has_shop = self.shops.values().any(|s| s.intersection_id == *id);

            grid[row][col] = if has_apartment {
                'A'
            } else if has_factory {
                'F'
            } else if has_shop {
                'S'
            } else {
                '+'
            };
        }

        // Draw cars
        for car in self.cars.values() {
            let (row, col) = to_grid(car.position.x, car.position.z);
            if grid[row][col] == ' ' || grid[row][col] == '·' {
                grid[row][col] = 'C';
            }
        }

        // Print the grid
        println!("\n=== World Map ===");
        println!("Legend: A=Apartment, F=Factory, S=Shop, +=Intersection, C=Car, ·=Road");
        println!();
        for row in &grid {
            let line: String = row.iter().collect();
            println!("{}", line);
        }
        println!();
    }
}
