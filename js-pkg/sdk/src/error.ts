/** Metadata associated with a {@link YSweetError}. */
export type YSweetErrorPayload =
  | { code: 'ServerRefused'; address: string; port: number; url: string }
  | { code: 'ServerError'; status: number; message: string; url: string }
  | { code: 'NoAuthProvided' }
  | { code: 'InvalidAuthProvided' }
  | { code: 'Unknown'; message: string }

/** An error returned by the y-sweet SDK. */
export class YSweetError extends Error {
  /**
   * Create a new {@link YSweetError}.
   *
   * @param cause An object representing metadata associated with the error.
   * @see {@link YSweetErrorPayload}
   */
  constructor(public cause: YSweetErrorPayload) {
    super(YSweetError.getMessage(cause))
    this.name = 'YSweetError'
  }

  /** Convert the message to an error string that can be displayed to the user.
   *
   * The error string can also be used with {@link YSweetError.fromMessage} to
   * reconstruct the payload object, which is useful in the context of Next.js,
   * which will only pass an error string from the server to the client.
   *
   * @param payload The payload object to convert to a string.
   * @returns A string representation of the error.
   */
  static getMessage(payload: YSweetErrorPayload): string {
    let message
    if (payload.code === 'ServerRefused') {
      message = `Server at ${payload.address}:${payload.port} refused connection. URL: ${payload.url}`
    } else if (payload.code === 'ServerError') {
      message = `Server responded with ${payload.status} ${payload.message}. URL: ${payload.url}`
    } else if (payload.code === 'NoAuthProvided') {
      message = 'No auth provided'
    } else if (payload.code === 'InvalidAuthProvided') {
      message = 'Invalid auth provided'
    } else {
      message = payload.message
    }
    return `${payload.code}: ${message}`
  }

  /**
   * In development, next.js passes error objects to the client but strips out everything but the
   * `message` field. This method allows us to reconstruct the original error object.
   *
   * @param messageString The error message string to reconstruct a payload from.
   * @returns A {@link YSweetError} object.
   * @see {@link https://nextjs.org/docs/app/api-reference/file-conventions/error#errormessage| Next.js docs}
   */
  static fromMessage(messageString: string): YSweetError {
    let match = messageString.match(/^(.*?): (.*)$/)
    if (!match) {
      return new YSweetError({ code: 'Unknown', message: messageString })
    }

    let [, code, message] = match

    if (code === 'ServerRefused') {
      match = message.match(/^Server at (.*?):(\d+) refused connection. URL: (.*)$/)
      if (!match) {
        return new YSweetError({ code: 'Unknown', message: messageString })
      }

      let [, address, port, url] = match
      return new YSweetError({ code, address, port: parseInt(port), url })
    }

    if (code === 'ServerError') {
      match = message.match(/^Server responded with (\d+) (.*). URL: (.*)$/)
      if (!match) {
        return new YSweetError({ code: 'Unknown', message: messageString })
      }

      let [, status, statusText, url] = match
      return new YSweetError({ code, status: parseInt(status), message: statusText, url })
    }

    if (code === 'NoAuthProvided') {
      return new YSweetError({ code })
    }

    if (code === 'InvalidAuthProvided') {
      return new YSweetError({ code })
    }

    return new YSweetError({ code: 'Unknown', message })
  }
}
