// Command nodes are not executable in offline mode.
export function CommandView() {
  return (
    <div className="editor">
      <button type="button" disabled>
        Execute
      </button>
      <div className="editor__hint">Offline mode — command disabled.</div>
    </div>
  );
}
