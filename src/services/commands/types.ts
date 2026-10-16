export type Modifier = "Primary" | "Control" | "Meta" | "Alt" | "Shift";
export interface Binding {
  id: string;
  commandId: string;
  scope: "application" | "terminal";
  context: "workspace" | "terminal";
  trigger:
    | { type: "key"; key: string; modifiers: Modifier[] }
    | {
        type: "pointer";
        button: "primary";
        event: "click";
        modifiers: Modifier[];
      };
  platform: "all";
  enabled: boolean;
}
