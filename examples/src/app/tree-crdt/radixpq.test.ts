import { RadixPriorityQueueBuilder } from './radixpq'

test('empty queue', () => {
  let builder = new RadixPriorityQueueBuilder<number>()
  let queue = builder.build()
  expect(queue[Symbol.iterator]().next()).toEqual({ value: undefined, done: true })
})

test('unique priorities', () => {
  let builder = new RadixPriorityQueueBuilder<number>()
  builder.addEntry(6, 1)
  builder.addEntry(10, 2)
  builder.addEntry(4, 3)
  let queue = builder.build()
  let result = Array.from(queue)
  expect(result).toEqual([2, 1, 3])
})

test('non-unique priorities use insertion order', () => {
  let builder = new RadixPriorityQueueBuilder<number>()
  builder.addEntry(6, 1)
  builder.addEntry(6, 2)
  builder.addEntry(6, 3)
  builder.addEntry(7, 4)
  builder.addEntry(7, 5)
  builder.addEntry(7, 6)
  let queue = builder.build()
  let result = Array.from(queue)
  expect(result).toEqual([4, 5, 6, 1, 2, 3])
})
