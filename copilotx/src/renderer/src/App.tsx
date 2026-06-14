import { useState, useEffect, useRef } from 'react'
import { TitleBar } from './TitleBar'
import { AnswerPanel } from './AnswerPanel'
import './styles.css'

interface Answer {
  id: number
  content: string
  error?: string
}

export type OverlayState = 'idle' | 'processing' | 'streaming' | 'error'

export default function App() {
  const [state, setState] = useState<OverlayState>('idle')
  const [answers, setAnswers] = useState<Answer[]>([])
  const [currentIndex, setCurrentIndex] = useState(0)
  const [streamingContent, setStreamingContent] = useState('')
  const [errorMessage, setErrorMessage] = useState('')
  const streamingRef = useRef(streamingContent)
  const answersLengthRef = useRef(answers.length)
  streamingRef.current = streamingContent
  answersLengthRef.current = answers.length

  useEffect(() => {
    window.api.onToken((content: string) => {
      setState('streaming')
      setStreamingContent((prev) => prev + content)
    })

    window.api.onCaptureState((newState: string, error?: string) => {
      if (newState === 'processing') {
        setState('processing')
        setStreamingContent('')
      } else if (newState === 'done') {
        setAnswers((prev) => [
          ...prev,
          { id: prev.length, content: streamingRef.current }
        ])
        setCurrentIndex(answersLengthRef.current)
        setStreamingContent('')
        setState('idle')
      } else if (newState === 'error') {
        setState('error')
        setErrorMessage(error || 'Unknown error')
      }
    })
  }, [])

  const handlePrev = () => {
    if (currentIndex > 0) setCurrentIndex(currentIndex - 1)
  }

  const handleNext = () => {
    if (currentIndex < answers.length - 1) setCurrentIndex(currentIndex + 1)
  }

  const displayContent =
    state === 'streaming'
      ? streamingContent
      : answers[currentIndex]?.content || ''

  return (
    <div className={`overlay ${state === 'error' ? 'error' : ''}`}>
      <TitleBar state={state} onClose={() => window.api.close()} />
      <AnswerPanel
        content={displayContent}
        state={state}
        errorMessage={errorMessage}
      />
      {answers.length > 1 && state === 'idle' && (
        <div className="navigation">
          <button onClick={handlePrev} disabled={currentIndex === 0}>
            &#9664;
          </button>
          <span className="counter">
            {currentIndex + 1} / {answers.length}
          </span>
          <button onClick={handleNext} disabled={currentIndex === answers.length - 1}>
            &#9654;
          </button>
        </div>
      )}
    </div>
  )
}
