import type { OverlayState } from './App'

interface AnswerPanelProps {
  content: string
  state: OverlayState
  errorMessage: string
}

export function AnswerPanel({ content, state, errorMessage }: AnswerPanelProps) {
  if (state === 'idle' && !content) {
    return (
      <div className="answer-panel idle">
        Press [hotkey] to capture
      </div>
    )
  }

  if (state === 'processing') {
    return (
      <div className="answer-panel processing">
        <div className="pulse-border" />
        Capturing screen...
      </div>
    )
  }

  if (state === 'error') {
    return (
      <div className="answer-panel error">
        Error: {errorMessage}
      </div>
    )
  }

  return (
    <div className="answer-panel streaming">
      <pre className="answer-text">{content}</pre>
    </div>
  )
}
