interface ViewerToolbarProps {
  zoomLabel?: string;
  deviceName?: string;
  onResetZoom: () => void;
  onSnapshot?: () => void;
  showHistogram?: boolean;
  onToggleHistogram?: () => void;
}

export function ViewerToolbar({ zoomLabel, deviceName, onResetZoom, onSnapshot, showHistogram, onToggleHistogram }: ViewerToolbarProps) {
  const titleClass = deviceName
    ? "iv-toolbar__title iv-toolbar__title--connected"
    : "iv-toolbar__title";

  return (
    <div className="iv-toolbar">
      <span className={titleClass}>{deviceName ?? "Image Viewer"}</span>
      <span className="iv-toolbar__spacer" />
      <button
        type="button"
        className="iv-toolbar__btn"
        onClick={onResetZoom}
        title="Fit to window"
        aria-label="Fit to window"
      >
        ⤢
      </button>
      {zoomLabel !== undefined && (
        <button
          type="button"
          className="iv-toolbar__zoom"
          onClick={onResetZoom}
          title="Reset zoom"
          aria-label="Reset zoom"
        >
          {zoomLabel}
        </button>
      )}
      {onToggleHistogram !== undefined && (
        <button
          type="button"
          className={`iv-toolbar__btn${showHistogram ? " iv-toolbar__btn--active" : ""}`}
          onClick={onToggleHistogram}
          title="Toggle histogram"
          aria-label="Toggle histogram"
          aria-pressed={showHistogram}
        >
          ▤
        </button>
      )}
      <button
        type="button"
        className="iv-toolbar__btn"
        disabled={!onSnapshot}
        onClick={onSnapshot}
        title="Save snapshot"
        aria-label="Save snapshot"
      >
        ⊙
      </button>
    </div>
  );
}
