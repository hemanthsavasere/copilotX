interface TextInputBarProps {
  text: string
  isActive: boolean
}

export function TextInputBar({ text, isActive }: TextInputBarProps) {
  if (!isActive) return null
  return (
    <div className="text-input-bar">
      <span className="input-text">{text}</span>
      <span className="input-cursor">|</span>
    </div>
  )
}
