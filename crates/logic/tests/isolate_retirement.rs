// Copyright 2026 Deno Land Inc. Apache-2.0 license.

use celld_logic::isolate::{
    admit, may_free, place, retire, IsolateLoad, Placement, PoolLimits, PoolLoad,
};

fn pool(min_isolates: usize, isolates: Vec<IsolateLoad>) -> PoolLoad {
    PoolLoad {
        isolates,
        limits: PoolLimits {
            grow_at: 2,
            shrink_under: 1,
            min_isolates,
            max_stateless: 4,
            max_requests: None,
            max_cells: 32,
        },
    }
}

#[test]
fn zero_floor_releases_the_last_idle_isolate_and_rebuilds_on_demand() {
    let mut load = pool(0, vec![IsolateLoad::default()]);

    assert_eq!(retire(&load), Some(0));
    load.isolates[0].retiring = true;

    assert!(may_free(&load.isolates[0]));
    assert_eq!(retire(&load), None);
    assert_eq!(place(&load), Placement::Grow);
}

#[test]
fn one_floor_preserves_the_last_idle_isolate_for_new_work() {
    let load = pool(1, vec![IsolateLoad::default()]);

    assert_eq!(retire(&load), None);
    assert_eq!(place(&load), Placement::Existing(0));
}

#[test]
fn one_floor_still_shrinks_a_burst_back_to_one_isolate() {
    let mut load = pool(1, vec![IsolateLoad::default(); 2]);

    let victim = retire(&load).expect("one surplus isolate can retire");
    load.isolates[victim].retiring = true;
    let survivor = 1 - victim;

    assert!(may_free(&load.isolates[victim]));
    assert_eq!(retire(&load), None);
    assert_eq!(place(&load), Placement::Existing(survivor));
}

#[test]
fn a_draining_isolate_does_not_satisfy_the_warm_floor() {
    let load = pool(
        1,
        vec![
            IsolateLoad {
                turns: 1,
                requests: 1,
                retiring: true,
                ..IsolateLoad::default()
            },
            IsolateLoad::default(),
        ],
    );

    assert_eq!(retire(&load), None);
    assert_eq!(place(&load), Placement::Existing(1));
}

#[test]
fn an_empty_pool_can_start_even_when_it_has_a_warm_floor() {
    for floor in [0, 1] {
        let load = pool(floor, Vec::new());

        assert_eq!(retire(&load), None);
        assert_eq!(admit(&load, false), Ok(Placement::Grow));
        assert_eq!(admit(&load, true), Ok(Placement::Grow));
    }
}

#[test]
fn a_surplus_isolate_retires_only_when_the_survivor_can_absorb_demand() {
    for floor in [0, 1] {
        let mut load = pool(
            floor,
            vec![
                IsolateLoad {
                    turns: 2,
                    requests: 2,
                    ..IsolateLoad::default()
                },
                IsolateLoad::default(),
            ],
        );

        assert_eq!(retire(&load), None);
        load.isolates[0].turns = 1;
        assert_eq!(retire(&load), Some(1));
    }
}

#[test]
fn resident_cells_remain_protected_independently_of_the_floor() {
    for floor in [0, 1] {
        let housed = IsolateLoad {
            cells: 1,
            ..IsolateLoad::default()
        };
        let mut load = pool(floor, vec![housed; 2]);

        assert_eq!(retire(&load), None);
        load.isolates.push(IsolateLoad::default());
        assert_eq!(retire(&load), Some(2));
    }
}

#[test]
fn a_suspended_request_prevents_freeing_after_voluntary_retirement() {
    let mut load = pool(
        0,
        vec![IsolateLoad {
            requests: 1,
            ..IsolateLoad::default()
        }],
    );

    assert_eq!(retire(&load), Some(0));
    load.isolates[0].retiring = true;
    assert!(!may_free(&load.isolates[0]));

    load.isolates[0].requests = 0;
    assert!(may_free(&load.isolates[0]));
}

#[test]
fn a_warm_floor_does_not_pin_a_condemned_or_superseded_isolate() {
    let mut load = pool(
        1,
        vec![IsolateLoad {
            turns: 1,
            requests: 1,
            cells: 1,
            retiring: true,
        }],
    );

    assert!(!may_free(&load.isolates[0]));
    load.isolates[0].turns = 0;
    assert!(!may_free(&load.isolates[0]));
    load.isolates[0].requests = 0;
    assert!(!may_free(&load.isolates[0]));
    load.isolates[0].cells = 0;
    assert!(may_free(&load.isolates[0]));
    assert_eq!(admit(&load, false), Ok(Placement::Grow));
    assert_eq!(admit(&load, true), Ok(Placement::Grow));
}
