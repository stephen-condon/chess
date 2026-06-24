export interface TrainingChallenge {
  id: string;
  title: string;
  description: string;
  fen: string;
}

export const CHALLENGES: TrainingChallenge[] = [
  {
    id: "rook",
    title: "Rook Fundamentals",
    description: "Checkmate the lone king using your rooks.",
    fen: "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1",
  },
  {
    id: "bishop",
    title: "Bishop Mastery",
    description: "Checkmate the lone king using your bishops.",
    fen: "4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1",
  },
  {
    id: "knight",
    title: "Knight Tactics",
    // ponytail: 2 knights can't force checkmate against optimal play, but medium AI will err
    description: "Checkmate the lone king using your knights.",
    fen: "4k3/8/8/8/8/8/8/1N2K1N1 w - - 0 1",
  },
  {
    id: "queen",
    title: "Queen Power",
    description: "Checkmate the lone king using your queen.",
    fen: "4k3/8/8/8/8/8/8/3QK3 w - - 0 1",
  },
];

const STORAGE_KEY = "chess-training-progress";

export function getCompleted(): string[] {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]") as string[];
  } catch {
    return [];
  }
}

export function markComplete(id: string): void {
  const completed = getCompleted();
  if (!completed.includes(id)) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify([...completed, id]));
  }
}

export function isUnlocked(challengeIndex: number): boolean {
  if (challengeIndex === 0) return true;
  return getCompleted().includes(CHALLENGES[challengeIndex - 1].id);
}
