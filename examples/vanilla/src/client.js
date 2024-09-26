import * as Y from 'yjs'
import { createYjsProvider } from '@y-sweet/client'
import { createColorGridUI } from './ColorGrid'

const COLORS = ['#500724', '#831843', '#9d174d', '#be185d', '#db2777', '#f472b6', '#f9a8d4', null]
const GRID_SIZE = 10
const QUERY_PARAM = 'doc'

async function main() {
  // First, fetch a client token that can access the docId in the URL.
  // Or, if the URL does not contain a docId, get a client token for a new doc.
  const url = new URL('http://localhost:9090/client-token')
  const searchParams = new URLSearchParams(window.location.search)
  const docId = searchParams.get(QUERY_PARAM)
  if (docId) url.searchParams.set(QUERY_PARAM, docId)
  const res = await fetch(url.toString())
  const clientToken = await res.json()

  // Update the URL to include the docId if it was not already present.
  if (!docId) {
    const url = new URL(window.location.href)
    url.searchParams.set(QUERY_PARAM, clientToken.docId)
    window.history.replaceState({}, '', url.toString())
  }

  // Create a Yjs document and connect it to the Y-Sweet server.
  const doc = new Y.Doc()
  createYjsProvider(doc, clientToken, { disableBc: true })
  const sharedColorMap = doc.getMap('colorgrid')

  // Create the UI for the color grid.
  const { colorGridEl, updateCell } = createColorGridUI(
    COLORS,
    GRID_SIZE,
    function onCellChange(key, color) {
      if (color === null) {
        sharedColorMap.delete(key)
      } else {
        sharedColorMap.set(key, color)
      }
    },
  )

  // Subscribe to changes on the sharedColorMap and update the UI accordingly.
  sharedColorMap.observe((event) => {
    event.keysChanged.forEach((key) => {
      updateCell(key, sharedColorMap.get(key) ?? null)
    })
  })

  // Add the color grid to the page.
  document.querySelector('main').appendChild(colorGridEl)
}

main().catch((err) => {
  console.error(err)
  showErrMsg()
})

function showErrMsg() {
  const el = document.querySelector('.error-message')
  el.classList.remove('hidden')
}
