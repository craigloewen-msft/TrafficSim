//! Main simulation world that ties everything together
//!
//! This is the entry point for running the traffic simulation
//! without any Bevy dependencies.

use anyhow::{Context, Result};
use ordered_float::OrderedFloat;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

use super::building::{
    SimFactory, SimHouse, SimShop, LABOR_DEMAND_THRESHOLD, PRODUCT_DEMAND_THRESHOLD,
};
use super::car::{CarUpdateResult, SimCar};
use super::intersection::SimIntersection;
use super::road_network::SimRoadNetwork;
use super::types::{
    CarId, FactoryId, HouseId, IntersectionId, Position, RoadId, ShopId, SimId, SimRoad,
};

/// Global demand metrics for the simulation
///
/// Tracks how many buildings have unmet demand - i.e., they need resources
/// but there isn't enough supply to meet their needs.
#[derive(Debug, Clone, Default)]
pub struct GlobalDemand {
    /// Number of factories waiting for workers but no houses have available cars
    pub factories_waiting: usize,
    /// Total number of factories
    pub total_factories: usize,
    /// Number of shops waiting for products but no factories have inventory
    pub shops_waiting: usize,
    /// Total number of shops
    pub total_shops: usize,
    /// Number of houses with available cars but no factories need workers
    pub houses_waiting: usize,
    /// Total number of houses
    pub total_houses: usize,
}

/// The main simulation world
pub struct SimWorld {
    /// Road network for pathfinding
    pub road_network: SimRoadNetwork,

    /// All intersections
    pub intersections: HashMap<IntersectionId, SimIntersection>,

    /// All cars
    pub cars: HashMap<CarId, SimCar>,

    /// All houses
    pub houses: HashMap<HouseId, SimHouse>,

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
}

impl Default for SimWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl SimWorld {
    pub fn new() -> Self {
        Self {
            road_network: SimRoadNetwork::new(),
            intersections: HashMap::new(),
            cars: HashMap::new(),
            houses: HashMap::new(),
            factories: HashMap::new(),
            shops: HashMap::new(),
            next_id: 0,
            time: 0.0,
            rng: None,
        }
    }

    /// Create a new SimWorld with a seeded RNG for reproducible simulations
    pub fn new_with_seed(seed: u64) -> Self {
        Self {
            road_network: SimRoadNetwork::new(),
            intersections: HashMap::new(),
            cars: HashMap::new(),
            houses: HashMap::new(),
            factories: HashMap::new(),
            shops: HashMap::new(),
            next_id: 0,
            time: 0.0,
            rng: Some(StdRng::seed_from_u64(seed)),
        }
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

    /// Add a house at an intersection
    pub fn add_house(&mut self, intersection_id: IntersectionId) -> HouseId {
        let id = HouseId(self.next_sim_id());
        let house = SimHouse::new(id, intersection_id);
        self.houses.insert(id, house);
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

    /// Remove a house from the world
    /// Returns the car that was associated with the house (if any)
    pub fn remove_house(&mut self, house_id: HouseId) -> Option<CarId> {
        let house = self.houses.remove(&house_id)?;
        house.car
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
        let houses_to_remove: Vec<HouseId> = self
            .houses
            .iter()
            .filter(|(_, h)| h.intersection_id == intersection_id)
            .map(|(id, _)| *id)
            .collect();

        for house_id in houses_to_remove {
            self.remove_house(house_id);
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
        self.cars.remove(&car_id);
        self.road_network.remove_car_from_tracking(car_id);

        // Clear house car reference
        for house in self.houses.values_mut() {
            if house.car == Some(car_id) {
                house.car = None;
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

    /// Spawn a car from a given intersection to a destination
    pub fn spawn_car(
        &mut self,
        from_intersection: IntersectionId,
        to_intersection: IntersectionId,
        origin_house: Option<IntersectionId>,
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

        // Generate random speed
        let speed = self.random_range(2.0..6.0);

        let id = CarId(self.next_sim_id());
        let car = SimCar::new(
            id,
            speed,
            road_id,
            from_intersection,
            path,
            origin_house,
            start_pos,
            road_angle,
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
                        results.push((car_id, CarUpdateResult::Despawn));
                    }
                    Ok(CarUpdateResult::ArrivedAtDestination(dest)) => {
                        results.push((car_id, CarUpdateResult::ArrivedAtDestination(dest)));
                    }
                    Err(_) => {
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
    fn update_shops(&mut self, delta_secs: f32) {
        for shop in self.shops.values_mut() {
            shop.update(delta_secs);
        }
    }

    /// Update all factories and get a list of products ready to ship
    fn update_factories(&mut self, delta_secs: f32) -> Vec<(FactoryId, IntersectionId)> {
        let mut products_ready = Vec::new();

        // Get shops needing products
        let shops_needing_products: Vec<IntersectionId> = self
            .shops
            .values()
            .filter(|s| s.product_demand >= PRODUCT_DEMAND_THRESHOLD)
            .map(|s| s.intersection_id)
            .collect();

        for factory in self.factories.values_mut() {
            factory.update(delta_secs);

            // Try to send products to shops
            for &shop_intersection in &shops_needing_products {
                if factory.take_product() {
                    products_ready.push((factory.id, shop_intersection));
                }
            }
        }

        products_ready
    }

    /// Spawn workers from houses to factories
    fn spawn_workers(&mut self) {
        // Get factories with high labor demand
        let factories_with_demand: Vec<(FactoryId, IntersectionId)> = self
            .factories
            .values()
            .filter(|f| f.labor_demand >= LABOR_DEMAND_THRESHOLD)
            .map(|f| (f.id, f.intersection_id))
            .collect();

        if factories_with_demand.is_empty() {
            return;
        }

        // Collect house IDs to process
        let house_ids: Vec<HouseId> = self.houses.keys().copied().collect();

        for house_id in house_ids {
            let house = match self.houses.get(&house_id) {
                Some(h) => h,
                None => continue,
            };

            // Only spawn if house doesn't have a car out
            if house.car.is_some() {
                continue;
            }

            // Choose random factory
            let (factory_id, factory_intersection) =
                match self.choose_random(&factories_with_demand) {
                    Some(&(fid, fi)) => (fid, fi),
                    None => continue,
                };

            // Try to reserve at factory
            if let Some(factory) = self.factories.get_mut(&factory_id) {
                if !factory.try_reserve_worker() {
                    continue;
                }
            } else {
                continue;
            }

            let house_intersection = self.houses.get(&house_id).map(|h| h.intersection_id);
            let house_intersection = match house_intersection {
                Some(hi) => hi,
                None => continue,
            };

            // Spawn car
            match self.spawn_car(
                house_intersection,
                factory_intersection,
                Some(house_intersection),
            ) {
                Ok(car_id) => {
                    if let Some(house) = self.houses.get_mut(&house_id) {
                        house.car = Some(car_id);
                    }
                }
                Err(_) => continue,
            }
        }
    }

    /// Main simulation tick
    pub fn tick(&mut self, delta_secs: f32) {
        self.time += delta_secs;

        // Update intersections
        self.update_intersections(delta_secs);

        // Update shops
        self.update_shops(delta_secs);

        // Update factories and get products to ship
        let products_to_ship = self.update_factories(delta_secs);

        // Spawn delivery cars for products
        for (factory_id, shop_intersection) in products_to_ship {
            if let Some(factory) = self.factories.get(&factory_id) {
                let factory_intersection = factory.intersection_id;
                let _ = self.spawn_car(factory_intersection, shop_intersection, None);
            }
        }

        // Spawn workers from houses
        self.spawn_workers();

        // Update cars and process results
        let car_results = self.update_cars(delta_secs);

        // Process car arrivals
        for (car_id, result) in car_results {
            match result {
                CarUpdateResult::ArrivedAtDestination(dest) => {
                    // Check if arrived at factory
                    let factory_id = self
                        .factories
                        .values()
                        .find(|f| f.intersection_id == dest)
                        .map(|f| f.id);

                    if let Some(fid) = factory_id {
                        // Pick a random shop as target
                        let shop_intersections: Vec<IntersectionId> =
                            self.shops.values().map(|s| s.intersection_id).collect();

                        let shop_intersection = self.choose_random(&shop_intersections).copied();
                        if let Some(shop_intersection) = shop_intersection {
                            if let Some(factory) = self.factories.get_mut(&fid) {
                                factory.receive_car(car_id, shop_intersection);
                            }
                        }
                    }

                    // Check if arrived at shop
                    if let Some(shop) = self.shops.values_mut().find(|s| s.intersection_id == dest)
                    {
                        shop.receive_delivery();
                    }

                    // Clear house car reference
                    for house in self.houses.values_mut() {
                        if house.car == Some(car_id) {
                            house.car = None;
                        }
                    }
                }
                CarUpdateResult::Despawn => {
                    // Clear house car reference
                    for house in self.houses.values_mut() {
                        if house.car == Some(car_id) {
                            house.car = None;
                        }
                    }
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
    fn build_test_world(mut world: SimWorld) -> Self {
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

        // Add houses (4 total) - offshoots from grid corners
        let house_data = vec![
            (grid[0][0], Position::new(-30.0, 0.0, -30.0)), // Top-left
            (grid[0][2], Position::new(30.0, 0.0, -30.0)),  // Top-right
            (grid[2][0], Position::new(-30.0, 0.0, 30.0)),  // Bottom-left
            (grid[2][2], Position::new(30.0, 0.0, 30.0)),   // Bottom-right
        ];

        for (grid_intersection, house_pos) in house_data {
            let house_intersection = world.add_intersection(house_pos);
            let _ = world.add_two_way_road(grid_intersection, house_intersection);
            world.add_house(house_intersection);
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
        println!("Houses: {}", self.houses.len());
        println!("Factories: {}", self.factories.len());
        println!("Shops: {}", self.shops.len());
        println!();

        // Factory status
        println!("--- Factories ---");
        for factory in self.factories.values() {
            println!(
                "  Factory {:?}: demand={:.1}, inventory={}/{}, processing={}",
                factory.id.0,
                factory.labor_demand,
                factory.inventory,
                factory.max_inventory,
                factory.processing_cars.len()
            );
        }

        // Shop status
        println!("--- Shops ---");
        for shop in self.shops.values() {
            println!(
                "  Shop {:?}: demand={:.1}, deliveries={}",
                shop.id.0, shop.product_demand, shop.cars_received
            );
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
            "  Houses waiting: {}/{}",
            demand.houses_waiting, demand.total_houses
        );
    }

    /// Calculate global demand metrics
    ///
    /// Returns metrics showing how many buildings have unmet demand:
    /// - Factories waiting: factories that need workers but no houses have available cars
    /// - Shops waiting: shops that need products but no factories have inventory
    /// - Houses waiting: houses with available cars but no factories need workers
    pub fn calculate_global_demand(&self) -> GlobalDemand {
        let total_factories = self.factories.len();
        let total_shops = self.shops.len();
        let total_houses = self.houses.len();

        // Count factories that need workers (labor_demand >= threshold)
        let factories_needing_workers: usize = self
            .factories
            .values()
            .filter(|f| f.labor_demand >= LABOR_DEMAND_THRESHOLD)
            .count();

        // Count houses with available cars (car is None)
        let houses_with_available_cars: usize =
            self.houses.values().filter(|h| h.car.is_none()).count();

        // Count shops that need products (product_demand >= threshold)
        let shops_needing_products: usize = self
            .shops
            .values()
            .filter(|s| s.product_demand >= PRODUCT_DEMAND_THRESHOLD)
            .count();

        // Count factories with inventory available
        let factories_with_inventory: usize =
            self.factories.values().filter(|f| f.inventory > 0).count();

        // Factories waiting: need workers but no available supply (no houses with cars)
        let factories_waiting = if houses_with_available_cars == 0 {
            factories_needing_workers
        } else {
            0
        };

        // Shops waiting: need products but no supply (no factories with inventory)
        let shops_waiting = if factories_with_inventory == 0 {
            shops_needing_products
        } else {
            0
        };

        // Houses waiting: have cars but no demand (no factories need workers)
        let houses_waiting = if factories_needing_workers == 0 {
            houses_with_available_cars
        } else {
            0
        };

        GlobalDemand {
            factories_waiting,
            total_factories,
            shops_waiting,
            total_shops,
            houses_waiting,
            total_houses,
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
            let has_house = self.houses.values().any(|h| h.intersection_id == *id);
            let has_factory = self.factories.values().any(|f| f.intersection_id == *id);
            let has_shop = self.shops.values().any(|s| s.intersection_id == *id);

            grid[row][col] = if has_house {
                'H'
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
        println!("Legend: H=House, F=Factory, S=Shop, +=Intersection, C=Car, ·=Road");
        println!();
        for row in &grid {
            let line: String = row.iter().collect();
            println!("{}", line);
        }
        println!();
    }
}
