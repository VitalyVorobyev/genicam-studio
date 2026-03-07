interface ViewerToolbarProps {
  zoomLabel?: string;
  deviceName?: string;
  onResetZoom: () => void;
}

export function ViewerToolbar({ zoomLabel, deviceName, onResetZoom }: ViewerToolbarProps) {
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
      <button
        type="button"
        className="iv-toolbar__btn"
        disabled
        title="Save snapshot (coming soon)"
        aria-label="Save snapshot"
      >
        ⊙
      </button>
    </div>
  );
}
