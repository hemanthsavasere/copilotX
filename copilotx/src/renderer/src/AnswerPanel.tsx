import type { OverlayState } from './App'

interface AnswerPanelProps {
  content: string
  state: OverlayState
  errorMessage: string
  dimmed?: boolean
}

export function AnswerPanel({ content, state, errorMessage, dimmed }: AnswerPanelProps) {
  if (state === 'idle' && !content) {
    return (
      <div className={`answer-panel idle${dimmed ? ' dimmed' : ''}`}>
        Press [hotkey] to capture
      </div>
    )
  }

  if (state === 'processing') {
    return (
      <div className={`answer-panel processing${dimmed ? ' dimmed' : ''}`}>
        <div className="pulse-border" />
        Capturing screen...
      </div>
    )
  }

  if (state === 'error') {
    return (
      <div className={`answer-panel error${dimmed ? ' dimmed' : ''}`}>
        Error: {errorMessage}
      </div>
    )
  }

  return (
    <div className={`answer-panel streaming${dimmed ? ' dimmed' : ''}`}>
      <pre className="answer-text">{content}</pre>
    </div>
  )
}
