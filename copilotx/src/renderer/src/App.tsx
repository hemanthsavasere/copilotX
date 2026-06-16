import { useState, useEffect, useRef } from 'react'
import { TitleBar } from './TitleBar'
import { AnswerPanel } from './AnswerPanel'
import { TextInputBar } from './TextInputBar'
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
  const [inputModeActive, setInputModeActive] = useState(false)
  const [inputText, setInputText] = useState('')
  const streamingRef = useRef(streamingContent)
  const answersLengthRef = useRef(answers.length)
  const inputModeActiveRef = useRef(inputModeActive)
  const inputTextRef = useRef(inputText)
  streamingRef.current = streamingContent
  answersLengthRef.current = answers.length
  inputModeActiveRef.current = inputModeActive
  inputTextRef.current = inputText

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

    window.api.onKeyEvent((key: string, _shift: boolean, ctrl: boolean) => {
      if (!inputModeActiveRef.current) return

      if (ctrl) return

      if (key === 'Enter') {
        if (inputTextRef.current.trim()) {
          window.api.sendTextInput(inputTextRef.current)
          setInputText('')
          setInputModeActive(false)
          setState('processing')
          setStreamingContent('')
        } else {
          window.api.stopInputMode()
          setInputModeActive(false)
          setInputText('')
        }
        return
      }

      if (key === 'Escape') {
        window.api.stopInputMode()
        setInputModeActive(false)
        setInputText('')
        return
      }

      if (key === 'Backspace') {
        setInputText((prev) => prev.slice(0, -1))
        return
      }

      setInputText((prev) => prev + key)
    })

    window.api.onInputModeState((newState: string) => {
      if (newState === 'active') {
        setInputModeActive(true)
        setInputText('')
      } else if (newState === 'inactive' || newState === 'error') {
        setInputModeActive(false)
        setInputText('')
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
    <div className={`overlay ${state === 'error' ? 'error' : ''} ${inputModeActive ? 'input-mode' : ''}`}>
      <TitleBar state={inputModeActive ? 'processing' : state} onClose={() => window.api.close()} />
      <AnswerPanel
        content={displayContent}
        state={state}
        errorMessage={errorMessage}
        dimmed={inputModeActive}
      />
      <TextInputBar text={inputText} isActive={inputModeActive} />
      {answers.length > 1 && state === 'idle' && !inputModeActive && (
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
