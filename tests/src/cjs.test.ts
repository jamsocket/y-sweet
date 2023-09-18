import { expect, test } from 'vitest'

test('Can import SDK via require', () => {
  const sdk = require('@y-sweet/sdk')

  expect(sdk).toBeDefined()
  expect(sdk.DocumentManager).toBeDefined()
})

test('Can import React library via require', () => {
  const react = require('@y-sweet/react')

  expect(react).toBeDefined()
  expect(react.useYDoc).toBeDefined()
})

test('Can import client library via require', () => {
  const client = require('@y-sweet/client')

  expect(client).toBeDefined()
  expect(client.createYjsProvider).toBeDefined()
})
