// Mirrors docs/DESIGN.md color tokens (canvas can't read CSS variables cheaply per frame).
export const C = {
  bg: '#0e0f13',
  floor: '#14161d',
  floorAlt: '#171922',
  wall: '#3a3f4f',
  wallTop: '#4a5064',
  players: ['#4fd1ff', '#ffcf3f', '#ff5fa2', '#7dff6a'] as const,
  walker: '#8f9a6e',
  runner: '#b0866a',
  danger: '#ff3b3b',
  shot: '#fff3b0',
  fg: '#f1f1f1',
  dim: '#8a8f9c',
} as const;
