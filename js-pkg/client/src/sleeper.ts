/**
 * A timeout that can be woken up prematurely by calling `wake()`.
 */
export class Sleeper {
  resolve?: () => void
  reject?: () => void
  promise: Promise<void>

  constructor(timeout: number) {
    this.promise = new Promise<void>((resolve, reject) => {
      setTimeout(() => {
        resolve()
      }, timeout)

      this.resolve = resolve
      this.reject = reject
    })
  }

  /**
   * Sleeps until the timeout has completed (or has been woken).
   */
  async sleep() {
    await this.promise
  }

  /**
   * Wakes up the timeout if it has not completed yet.
   */
  wake() {
    this.resolve && this.resolve()
  }
}
