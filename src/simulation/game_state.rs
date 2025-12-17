//! Game state tracking for the traffic management game
//!
//! This module tracks the player's resources, score, and objectives
//! to turn the traffic simulation into a fun management game.

/// Building costs for the game
pub const COST_ROAD: i32 = 50;
pub const COST_HOUSE: i32 = 200;
pub const COST_FACTORY: i32 = 500;
pub const COST_SHOP: i32 = 300;

/// Revenue from successful operations
pub const REVENUE_WORKER_DELIVERY: i32 = 10;  // Worker completes shift
pub const REVENUE_SHOP_DELIVERY: i32 = 50;     // Truck delivers to shop

/// Starting budget for the player
pub const STARTING_BUDGET: i32 = 2000;

/// Game objectives and completion thresholds
pub const GOAL_DELIVERIES: usize = 50;        // Deliveries needed to win
pub const GOAL_MONEY: i32 = 5000;              // Money target to win

/// Game state that tracks player progress and resources
#[derive(Debug, Clone)]
pub struct GameState {
    /// Player's current money
    pub money: i32,
    
    /// Total worker trips completed (house -> factory -> house)
    pub worker_trips_completed: usize,
    
    /// Total shop deliveries completed (factory -> shop -> factory)
    pub shop_deliveries_completed: usize,
    
    /// Game time in seconds
    pub time: f32,
    
    /// Whether the game is won
    pub is_won: bool,
    
    /// Whether the game is lost (bankrupt)
    pub is_lost: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Create a new game state with starting conditions
    pub fn new() -> Self {
        Self {
            money: STARTING_BUDGET,
            worker_trips_completed: 0,
            shop_deliveries_completed: 0,
            time: 0.0,
            is_won: false,
            is_lost: false,
        }
    }
    
    /// Check if player can afford a purchase
    pub fn can_afford(&self, cost: i32) -> bool {
        self.money >= cost
    }
    
    /// Deduct money for a purchase
    /// Returns true if successful, false if insufficient funds
    pub fn spend(&mut self, cost: i32) -> bool {
        if self.can_afford(cost) {
            self.money -= cost;
            true
        } else {
            false
        }
    }
    
    /// Add money from revenue
    pub fn earn(&mut self, amount: i32) {
        self.money += amount;
    }
    
    /// Record a worker trip completion and award revenue
    pub fn complete_worker_trip(&mut self) {
        self.worker_trips_completed += 1;
        self.earn(REVENUE_WORKER_DELIVERY);
    }
    
    /// Record a shop delivery completion and award revenue
    pub fn complete_shop_delivery(&mut self) {
        self.shop_deliveries_completed += 1;
        self.earn(REVENUE_SHOP_DELIVERY);
    }
    
    /// Update game time and check win/loss conditions
    pub fn update(&mut self, delta_secs: f32) {
        self.time += delta_secs;
        
        // Check win conditions
        if self.shop_deliveries_completed >= GOAL_DELIVERIES || self.money >= GOAL_MONEY {
            self.is_won = true;
        }
        
        // Check loss condition (bankrupt with no way to recover)
        // Player is only truly bankrupt if they can't afford the cheapest item
        if self.money < 0 {
            self.is_lost = true;
        }
    }
    
    /// Get total deliveries (workers + shop)
    pub fn total_deliveries(&self) -> usize {
        self.worker_trips_completed + self.shop_deliveries_completed
    }
    
    /// Get a summary string for display
    pub fn summary(&self) -> String {
        format!(
            "Money: ${} | Worker Trips: {} | Shop Deliveries: {} | Time: {:.1}s",
            self.money, self.worker_trips_completed, self.shop_deliveries_completed, self.time
        )
    }
    
    /// Get progress towards goals as a percentage
    pub fn goal_progress(&self) -> (f32, f32) {
        let delivery_progress = (self.shop_deliveries_completed as f32 / GOAL_DELIVERIES as f32 * 100.0).min(100.0);
        let money_progress = (self.money as f32 / GOAL_MONEY as f32 * 100.0).min(100.0);
        (delivery_progress, money_progress)
    }
}
