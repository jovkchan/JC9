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

export interface ProcessOutput {
  processId: string;
  type: 'stdout' | 'stderr';
  data: string;
}

export interface ProcessStarted {
  processId: string;
  pid: number;
}

export type RunningStatus = 'stopped' | 'running';
