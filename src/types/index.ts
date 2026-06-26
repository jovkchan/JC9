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
