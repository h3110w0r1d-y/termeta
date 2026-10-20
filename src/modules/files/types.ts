import type { SessionState } from "@/services/desktop/types";

export interface SftpEndpoint {
  profileId: string | null;
  sessionId?: string;
  state: SessionState;
  error: string;
  opening: boolean;
}
export interface SftpSelection {
  sessionId: string;
  path: string;
  names: string[];
}
