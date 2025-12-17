//! Game mechanics validation test
//!
//! This test validates that the game mechanics work correctly

use traffic_sim::simulation::{
    GameState, SimWorld, COST_HOUSE, COST_ROAD, GOAL_DELIVERIES, GOAL_MONEY,
    Position, REVENUE_SHOP_DELIVERY, REVENUE_WORKER_DELIVERY, STARTING_BUDGET,
    COMMUTE_HEALTHY_DISTANCE, SHORT_COMMUTE_PENALTY,
};

#[test]
fn test_game_state_initialization() {
    let game_state = GameState::new();
    assert_eq!(game_state.money, STARTING_BUDGET);
    assert_eq!(game_state.worker_trips_completed, 0);
    assert_eq!(game_state.shop_deliveries_completed, 0);
    assert!(!game_state.is_won);
    assert!(!game_state.is_lost);
}

#[test]
fn test_game_state_revenue() {
    let mut game_state = GameState::new();
    let initial_money = game_state.money;

    // Complete a worker trip
    game_state.complete_worker_trip(COMMUTE_HEALTHY_DISTANCE + 5.0);
    assert_eq!(game_state.worker_trips_completed, 1);
    assert_eq!(
        game_state.money,
        initial_money + REVENUE_WORKER_DELIVERY
    );

    // Complete a shop delivery
    let money_before = game_state.money;
    game_state.complete_shop_delivery();
    assert_eq!(game_state.shop_deliveries_completed, 1);
    assert_eq!(game_state.money, money_before + REVENUE_SHOP_DELIVERY);
}

#[test]
fn test_game_state_spending() {
    let mut game_state = GameState::new();
    let initial_money = game_state.money;

    // Can afford and spend
    assert!(game_state.can_afford(COST_ROAD));
    assert!(game_state.spend(COST_ROAD));
    assert_eq!(game_state.money, initial_money - COST_ROAD);

    // Cannot afford expensive item
    assert!(!game_state.can_afford(100000));
    assert!(!game_state.spend(100000));
    assert_eq!(game_state.money, initial_money - COST_ROAD); // Money unchanged
}

#[test]
fn test_game_world_with_state() {
    let world = SimWorld::new_with_game();
    assert!(world.game_state.is_some());

    if let Some(game_state) = &world.game_state {
        assert_eq!(game_state.money, STARTING_BUDGET);
    }
}

#[test]
fn test_game_world_building_costs() {
    let mut world = SimWorld::new_with_game();

    // Create an intersection to place a house on
    let pos = Position::new(0.0, 0.0, 0.0);
    let intersection_id = world.add_intersection(pos);

    let initial_money = world.game_state.as_ref().unwrap().money;

    // Try to add a house (should succeed and cost money)
    let maybe_house_id = world.try_add_house(intersection_id);
    assert!(maybe_house_id.is_some());
    assert_eq!(
        world.game_state.as_ref().unwrap().money,
        initial_money - COST_HOUSE
    );
}

#[test]
fn test_game_world_building_costs_block_when_broke() {
    let mut world = SimWorld::new_with_game();
    if let Some(game_state) = world.game_state.as_mut() {
        game_state.money = 10;
    }

    let first = Position::new(0.0, 0.0, 0.0);
    let second = Position::new(10.0, 0.0, 0.0);
    let start = world.add_intersection(first);
    let end = world.add_intersection(second);

    assert!(world.try_add_house(start).is_none());
    assert!(world
        .try_add_two_way_road(start, end)
        .expect("road creation should not error")
        .is_none());
}

#[test]
fn test_win_condition_deliveries() {
    let mut game_state = GameState::new();

    // Complete enough deliveries to win
    for _ in 0..GOAL_DELIVERIES {
        game_state.complete_shop_delivery();
    }

    game_state.update(0.1);
    assert!(game_state.is_won);
}

#[test]
fn test_win_condition_money() {
    let mut game_state = GameState::new();

    // Earn enough money to win
    game_state.earn(GOAL_MONEY);

    game_state.update(0.1);
    assert!(game_state.is_won);
}

#[test]
fn test_lose_condition() {
    let mut game_state = GameState::new();

    // Spend all money and go bankrupt
    game_state.money = -100;

    game_state.update(0.1);
    assert!(game_state.is_lost);
}

#[test]
fn test_short_commute_penalty_applied() {
    let mut game_state = GameState::new();
    let initial_money = game_state.money;

    game_state.complete_worker_trip(0.0);

    let expected_penalty = SHORT_COMMUTE_PENALTY;
    assert_eq!(
        game_state.money,
        initial_money + REVENUE_WORKER_DELIVERY - expected_penalty
    );
}
