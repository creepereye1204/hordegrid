//! Ammo, reload and splash damage (docs/product-specs/weapons-and-enemies.md).

use hg_sim::config::{EnemyKind, WeaponKind, ENEMIES, INFINITE_AMMO, RELOAD_FRAMES, WEAPONS};
use hg_sim::{Fixed, PlayerInput, World};

const HOLD_FIRE: PlayerInput = PlayerInput(PlayerInput::PRESENT | PlayerInput::FIRE);

#[test]
fn when_ammo_runs_out_then_auto_reloads_and_refills() {
    let world = World::new(0);
    let mut s = world.initial_state(1, 1, 0);
    let smg = WeaponKind::Smg as usize;
    s.wave.unlocked |= 1 << smg;
    s.players.weapon[0] = smg as u8;
    s.players.ammo[0][smg] = 1;

    let mut w = World::new(0);
    // First tick: fires the last round, ammo hits 0.
    w.step(
        &mut s,
        &[
            HOLD_FIRE,
            PlayerInput::default(),
            PlayerInput::default(),
            PlayerInput::default(),
        ],
    );
    assert_eq!(s.players.ammo[0][smg], 0);
    assert_eq!(s.players.reload_timer[0], 0, "not reloading yet this frame");

    // Next tick with fire still held and 0 ammo: auto-reload kicks in.
    w.step(
        &mut s,
        &[
            HOLD_FIRE,
            PlayerInput::default(),
            PlayerInput::default(),
            PlayerInput::default(),
        ],
    );
    assert!(
        s.players.reload_timer[0] > 0,
        "should start reloading on empty"
    );

    for _ in 0..u32::from(RELOAD_FRAMES) {
        w.step(&mut s, &[PlayerInput(PlayerInput::PRESENT); 4]);
    }
    assert_eq!(s.players.reload_timer[0], 0, "reload should finish");
    assert_eq!(
        s.players.ammo[0][smg], WEAPONS[smg].max_ammo,
        "ammo refilled to max"
    );
}

#[test]
fn when_pistol_then_never_reloads() {
    let world = World::new(0);
    let mut s = world.initial_state(1, 1, 0);
    assert_eq!(WEAPONS[WeaponKind::Pistol as usize].max_ammo, INFINITE_AMMO);

    let mut w = World::new(0);
    for _ in 0..600 {
        w.step(
            &mut s,
            &[
                HOLD_FIRE,
                PlayerInput::default(),
                PlayerInput::default(),
                PlayerInput::default(),
            ],
        );
        assert_eq!(
            s.players.reload_timer[0], 0,
            "pistol must never enter reload"
        );
    }
}

#[test]
fn when_rocket_hits_then_splash_damages_nearby_enemies_too() {
    let world = World::new(0);
    let mut s = world.initial_state(1, 1, 0);
    let rocket = WeaponKind::Rocket as usize;
    s.wave.unlocked |= 1 << rocket;
    s.players.weapon[0] = rocket as u8;
    s.players.ammo[0][rocket] = 1;
    s.players.facing[0] = 3; // east
    s.players.x[0] = Fixed::from_int(10) + Fixed::HALF;
    s.players.y[0] = Fixed::from_int(10) + Fixed::HALF;

    // Direct target, dead ahead.
    s.enemies.alive[0] = 1;
    s.enemies.kind[0] = EnemyKind::Walker as u8;
    s.enemies.hp[0] = ENEMIES[EnemyKind::Walker as usize].hp;
    s.enemies.x[0] = Fixed::from_int(12) + Fixed::HALF;
    s.enemies.y[0] = Fixed::from_int(10) + Fixed::HALF;

    // Bystander, close enough to be inside the splash radius but not directly hit.
    s.enemies.alive[1] = 1;
    s.enemies.kind[1] = EnemyKind::Walker as u8;
    s.enemies.hp[1] = ENEMIES[EnemyKind::Walker as usize].hp;
    s.enemies.x[1] = Fixed::from_int(12) + Fixed::HALF;
    s.enemies.y[1] = Fixed::from_int(11) + Fixed::HALF;

    let mut w = World::new(0);
    let inputs = [
        HOLD_FIRE,
        PlayerInput::default(),
        PlayerInput::default(),
        PlayerInput::default(),
    ];
    for _ in 0..30 {
        w.step(&mut s, &inputs);
        if s.enemies.hp[1] < ENEMIES[EnemyKind::Walker as usize].hp {
            break;
        }
    }
    assert!(
        s.enemies.hp[1] < ENEMIES[EnemyKind::Walker as usize].hp,
        "bystander enemy should take splash damage without being the direct hit"
    );
}
