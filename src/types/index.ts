export interface Command {
  id: string;
  name: string;
  command: string;
  workingDir: string;
}

export interface Project {
  id: string;
  name: string;
  commands: Command[];
  createdAt: string;
}
export type RunningStatus = 'stopped' | 'running';

// ── 工作流（多命令顺序执行，替代旧快捷方式）──

export interface WorkflowStep {
  name: string
  command: string
  workingDir: string
}

export interface Workflow {
  id: string
  name: string
  description: string
  category: string
  steps: WorkflowStep[]
  favorite?: boolean
  useCount?: number
}
