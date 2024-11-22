const SPINNER_FRAMES = [
  '☀️',
  '☀️',
  '☀️',
  '🌤',
  '🌥',
  '☁️',
  '🌧',
  '🌨',
  '🌧',
  '🌨',
  '🌧',
  '🌨',
  '⛈️',
  '🌨',
  '🌧',
  '🌨',
  '☁️',
  '🌥',
  '🌤',
  '☀️',
  '☀️',
]

/**
 * Creates a loading spinner with a message
 * @param {string} message
 * @returns {{ stop: () => void }}
 */
export function spinner(message) {
  let playing = true
  let frame = 0
  const interval = setInterval(() => {
    process.stdout.write(`\r${SPINNER_FRAMES[frame].padEnd(2)}  ${message}`)
    frame = (frame + 1) % SPINNER_FRAMES.length
  }, 100)

  return {
    stop: () => {
      if (!playing) return
      playing = false
      clearInterval(interval)
      process.stdout.write('\r' + ' '.repeat(message.length + 4) + '\r') // +4 for emoji + spaces
    },
  }
}

/**
 * Returns a string wrapped in ANSI escape codes for gray text.
 * @param {string} text
 */
export function gray(text) {
  return `\x1b[90m${text}\x1b[0m`
}

/**
 * Returns a string wrapped in ANSI escape codes for bold text.
 * @param {string} text
 */
export function bold(text) {
  return `\x1b[1m${text}\x1b[0m`
}
