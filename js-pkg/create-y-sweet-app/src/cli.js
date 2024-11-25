import readline from 'node:readline'

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

/**
 * @param {string} prompt
 * @param {string} defaultValue
 * @returns {Promise<string>}
 */
export function question(prompt, defaultValue = '') {
  return new Promise((resolve) => {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    })

    rl.question(prompt, (result) => {
      rl.close()
      resolve(result || defaultValue)
    })
  })
}

/**
 * Creates an interactive multiple choice selection menu without clearing console
 * @param {string} prompt - The question to display above choices
 * @param {string[]} choices - Array of options user can choose from
 * @returns {Promise<string>} - Returns selected choice
 */
export function select(prompt, choices) {
  return new Promise((resolve) => {
    let selected = 0
    let lines = 0 // track number of lines we've rendered

    function render() {
      // move cursor up to clear previous render
      if (lines > 0) process.stdout.write(`\x1B[${lines}A`)

      // render prompt and choices
      console.log(bold(prompt) + '\n')
      choices.forEach((choice, i) => {
        const indicator = i === selected ? '❯' : ' '
        console.log(`${indicator} ${choice}`)
      })

      // store number of lines we just rendered
      lines = choices.length + 2 // +2 for prompt and blank line
    }

    function cleanup() {
      // remove event listener
      process.stdin.off('data', onData)

      // clean up the menu
      process.stdout.write(`\x1B[${lines}A`) // Move up
      process.stdout.write(`\x1B[J`) // Clear to bottom

      // restore cursor and normal mode
      process.stdout.write('\x1B[?25h')
      process.stdin.setRawMode(false)
      process.stdin.pause()
    }

    /** @param {Buffer} data */
    function onData(data) {
      const key = data.toString()

      switch (key) {
        case '\u001b[A': {
          selected = (selected + choices.length - 1) % choices.length
          render()
          break
        }

        case '\u001b[B': {
          selected = (selected + 1) % choices.length
          render()
          break
        }

        case '\r': {
          cleanup()
          console.log(bold(prompt) + ' ' + choices[selected])
          resolve(choices[selected])
          break
        }

        case '\u0003': {
          cleanup()
          return process.exit(1)
        }
      }
    }

    // hide cursor and enable raw mode
    process.stdout.write('\x1B[?25l')
    process.stdin.setRawMode(true)
    process.stdin.resume()

    render()

    process.stdin.on('data', onData)
  })
}
