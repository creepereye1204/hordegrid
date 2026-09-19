// Contract with crates/web/src/view.rs (docs/generated/render-view.md). Keep in lockstep.
export const HEADER = 17;
export const H = {
  FRAME: 0,
  N_PLAYERS: 1,
  N_ENEMIES: 2,
  N_SHOTS: 3,
  WAVE: 4,
  /** Survival `Phase`, or `HideSeekPhase` when MODE === GameMode.HideSeek. */
  PHASE: 5,
  TIMER: 6,
  SCORE: 7,
  COMBO: 8,
  COMBO_TIMER: 9,
  UNLOCKED: 10,
  LOCAL: 11,
  NUM_PLAYERS: 12,
  BEST_COMBO: 13,
  TO_SPAWN: 14,
  MODE: 15,
  HS_WINNER: 16,
} as const;
export const PLAYER_STRIDE = 14;
export const P = {
  X: 0, Y: 1, HP: 2, LIFE: 3, FACING: 4, WEAPON: 5, HURT: 6, DOWN_TIMER: 7, REVIVE: 8, KILLS: 9,
  /** Hide & seek only: 0 hider / 1 seeker. */
  ROLE: 10,
  /** Hide & seek only: 1 once caught. */
  FOUND: 11,
  /** Ammo left in the equipped weapon; -1 = infinite (never reloads). */
  AMMO: 12,
  /** Frames left in an in-progress reload; 0 = not reloading. */
  RELOAD_TIMER: 13,
} as const;
export const ENEMY_STRIDE = 6;
export const E = { SLOT: 0, X: 1, Y: 2, KIND: 3, HP_PERMILLE: 4, STAGGER: 5 } as const;
export const SHOT_STRIDE = 4;
export const S = { X: 0, Y: 1, KIND: 2, DIR: 3 } as const;
export const MAX_PLAYERS = 4;
export const MAX_ENEMIES = 384;
export const FIXED_ONE = 65536;

export const Life = { Empty: 0, Alive: 1, Downed: 2, Dead: 3, Gone: 4 } as const;
export const Phase = { Intermission: 0, Active: 1, GameOver: 2 } as const;
export const GameMode = { Survival: 0, HideSeek: 1 } as const;
export const HideSeekPhase = { Hiding: 0, Seeking: 1, RoundOver: 2 } as const;
export const EventKind = {
  Hit: 1, Kill: 2, PlayerHurt: 3, Down: 4, Revive: 5, Unlock: 6, WaveStart: 7, GameOver: 8, Fire: 9,
  Found: 10, RoundEnd: 11, ReloadStart: 12, ReloadDone: 13,
} as const;
export const TickStatus = { Advanced: 0, Waiting: 1, Skipped: 2 } as const;
export const NetEventCode = { Synchronizing: 1, Synchronized: 2, Interrupted: 3, Resumed: 4, Disconnected: 5, Desync: 6 } as const;

export const WEAPON_NAMES = ['권총', '기관단총', '샷건', '로켓런처'] as const;
export const REVIVE_FRAMES = 120;
export const DOWNED_FRAMES = 600;
export const RELOAD_FRAMES = 60;

export const enemiesOffset = HEADER + MAX_PLAYERS * PLAYER_STRIDE;
export const shotsOffset = (nEnemies: number): number => enemiesOffset + nEnemies * ENEMY_STRIDE;
