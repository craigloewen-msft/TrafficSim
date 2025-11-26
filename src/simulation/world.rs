//! Main simulation world that ties everything together
//! 
//! This is the entry point for running the traffic simulation
//! without any Bevy dependencies.

use anyhow::{Context, Result};
use ordered_float::OrderedFloat;
use rand::Rng;
use rand::seq::IndexedRandom;
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
        let mut rng = rand::rng();
        let speed = rng.random_range(2.0..6.0);

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
                let result = car.update(delta_secs, &mut self.road_network, &mut self.intersections);

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

        let mut rng = rand::rng();

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
            let (factory_id, factory_intersection) = match factories_with_demand.choose(&mut rng) {
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
            match self.spawn_car(house_intersection, factory_intersection, Some(house_intersection)) {
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

                        if let Some(&shop_intersection) = shop_intersections.choose(&mut rand::rng())
                        {
                            if let Some(factory) = self.factories.get_mut(&fid) {
                                factory.receive_car(car_id, shop_intersection);
                            }
                        }
                    }

                    // Check if arrived at shop
                    if let Some(shop) = self
                        .shops
                        .values_mut()
                        .find(|s| s.intersection_id == dest)
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
        let mut world = SimWorld::new();

        // Create main intersections
        let bottom = world.add_intersection(Position::new(0.0, 0.0, 20.0));
        let top = world.add_intersection(Position::new(0.0, 0.0, -20.0));

        // Connect with two-way road
        let _ = world.add_two_way_road(bottom, top);

        // Add houses connected to bottom intersection
        let house_positions = vec![
            Position::new(-8.0, 0.0, 25.0),
            Position::new(-4.0, 0.0, 25.0),
            Position::new(0.0, 0.0, 26.0),
            Position::new(4.0, 0.0, 25.0),
            Position::new(8.0, 0.0, 25.0),
        ];

        for pos in house_positions {
            let house_intersection = world.add_intersection(pos);
            let _ = world.add_two_way_road(house_intersection, bottom);
            world.add_house(house_intersection);
        }

        // Add houses connected to top intersection
        let top_house_positions = vec![
            Position::new(0.0, 0.0, -25.0),
            Position::new(-4.0, 0.0, -25.0),
            Position::new(4.0, 0.0, -25.0),
        ];

        for pos in top_house_positions {
            let house_intersection = world.add_intersection(pos);
            let _ = world.add_two_way_road(house_intersection, top);
            world.add_house(house_intersection);
        }

        // Add factories connected to top intersection
        let factory_positions = vec![
            Position::new(-8.0, 0.0, -25.0),
            Position::new(8.0, 0.0, -25.0),
        ];

        for pos in factory_positions {
            let factory_intersection = world.add_intersection(pos);
            let _ = world.add_two_way_road(factory_intersection, top);
            world.add_factory(factory_intersection);
        }

        // Add shops connected to bottom intersection
        let shop_positions = vec![
            Position::new(-8.0, 0.0, 20.0),
            Position::new(8.0, 0.0, 20.0),
        ];

        for pos in shop_positions {
            let shop_intersection = world.add_intersection(pos);
            let _ = world.add_two_way_road(shop_intersection, bottom);
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
    }
}
