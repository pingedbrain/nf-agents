// Milestone versions and status, from ROADMAP.md. The landing page reads these so
// version numbers are written once. Scores stay `pending` until a run publishes them.

export type MilestoneStatus = 'unreleased' | 'planned';

export interface Milestone {
  version: string;
  title: string;
  status: MilestoneStatus;
}

export const milestones = {
  foundations: { version: 'v0.1', title: 'Foundations', status: 'unreleased' },
  agents: { version: 'v0.2', title: 'Agents & evals', status: 'planned' },
  benchmark: { version: 'v0.3', title: 'Benchmark', status: 'planned' },
  infrastructure: { version: 'v0.4', title: 'Infrastructure', status: 'planned' },
  compliance: { version: 'v0.5', title: 'Compliance profiles', status: 'planned' },
  ecosystem: { version: 'v0.6', title: 'Ecosystem', status: 'planned' },
} satisfies Record<string, Milestone>;

export const SCORE_PENDING = 'pending';
