import type { ImageRect } from "./roiUtils";
import { formatRoiLabel, roiIsValid } from "./roiUtils";

interface ViewerToolbarProps {
  zoomLabel?: string;
  deviceName?: string;
  onResetZoom: () => void;
  onSnapshot?: () => void;
  showHistogram?: boolean;
  onToggleHistogram?: () => void;
  showRoiTool?: boolean;
  onToggleRoiTool?: () => void;
  roiRect?: ImageRect | null;
  onApplyRoi?: () => void;
}

export function ViewerToolbar({
  zoomLabel,
  deviceName,
  onResetZoom,
  onSnapshot,
  showHistogram,
  onToggleHistogram,
  showRoiTool,
  onToggleRoiTool,
  roiRect,
  onApplyRoi,
}: ViewerToolbarProps) {
  const titleClass = deviceName
    ? "iv-toolbar__title iv-toolbar__title--connected"
    : "iv-toolbar__title";

  const hasValidRoi = roiRect !== null && roiRect !== undefined && roiIsValid(roiRect);

  return (
    <div className="iv-toolbar">
      <span className={titleClass}>{deviceName ?? "Image Viewer"}</span>
      <span className="iv-toolbar__spacer" />
      {/* ROI apply button — only visible when a valid selection exists */}
      {hasValidRoi && onApplyRoi !== undefined && (
        <button
          type="button"
          className="iv-toolbar__roi-apply"
          onClick={onApplyRoi}
          title={`Apply ROI: ${formatRoiLabel(roiRect!)}`}
          aria-label="Apply selected ROI"
        >
          Apply {roiRect!.w}×{roiRect!.h}
        </button>
      )}
      {onToggleRoiTool !== undefined && (
        <button
          type="button"
          className={`iv-toolbar__btn${showRoiTool ? " iv-toolbar__btn--active" : ""}`}
          onClick={onToggleRoiTool}
          title="ROI selection tool"
          aria-label="Toggle ROI selection tool"
          aria-pressed={showRoiTool}
        >
          ⊡
        </button>
      )}
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
