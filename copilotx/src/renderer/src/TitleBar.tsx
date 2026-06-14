import type { OverlayState } from './App'

interface TitleBarProps {
  state: OverlayState
  onClose: () => void
}

export function TitleBar({ state, onClose }: TitleBarProps) {
  return (
    <div className="title-bar">
      <span className={`status-dot ${state}`} />
      <span className="title-text">CopilotX</span>
      <button className="close-btn" onClick={onClose}>&#10005;</button>
    </div>
  )
}
