export class RadixPriorityQueueBuilder<T> {
  entries: Array<Array<T>> = []

  addEntry(priority: number, entry: T) {
    if (!this.entries[priority]) {
      this.entries[priority] = []
    }
    this.entries[priority].push(entry)
  }

  build(): RadixPriorityQueue<T> {
    return new RadixPriorityQueue<T>(this.entries)
  }
}

export class RadixPriorityQueue<T> {
  entriesReversed: Array<Array<T>> = []

  constructor(entries: Array<Array<T>> = []) {
    this.entriesReversed = entries.reverse()
  }

  [Symbol.iterator](): RadixPriorityQueueIterator<T> {
    return new RadixPriorityQueueIterator(this)
  }
}

export class RadixPriorityQueueIterator<T> {
  private iter: IterableIterator<[number, Array<T>]>
  private subIter: IterableIterator<[number, T]> | null

  constructor(private queue: RadixPriorityQueue<T>) {
    this.iter = queue.entriesReversed.entries()
    this.subIter = null
  }

  next(): { value: T; done: false } | { value: void; done: true } {
    while (true) {
      if (this.subIter) {
        const result = this.subIter.next()
        if (!result.done) {
          return { value: result.value[1], done: false }
        }
        this.subIter = null
      } else {
        const result = this.iter.next()
        if (result.done) {
          return { value: undefined, done: true }
        }
        this.subIter = result.value[1]?.entries()
      }
    }
  }
}
