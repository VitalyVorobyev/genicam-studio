interface CommandViewProps {
  canExecute: boolean;
  disabledReason: string;
  onExecute: () => void;
}

// Command nodes are not executable in offline mode; the button stays disabled
// until a live provider implements executeCommand.
export function CommandView({ canExecute, disabledReason, onExecute }: CommandViewProps) {
  const disabled = !canExecute;
  const title = disabled
    ? disabledReason || "Offline mode — command disabled."
    : "Execute command";

  return (
    <div className="editor">
      <button type="button" disabled={disabled} title={title} onClick={onExecute}>
        Execute
      </button>
      <div className="editor__hint">{title}</div>
    </div>
  );
}
