interface ViewerToolbarProps {
  zoomLabel?: string;
}

export function ViewerToolbar({ zoomLabel }: ViewerToolbarProps) {
  return (
    <div className="iv-toolbar">
      <span className="iv-toolbar__title">Image Viewer</span>
      {zoomLabel !== undefined && (
        <span className="iv-toolbar__zoom">{zoomLabel}</span>
      )}
    </div>
  );
}
