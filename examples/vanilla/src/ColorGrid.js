import './style.css'

// This file just implements the ColorGrid UI. It doesn't use Y-Sweet/Yjs at all.
// Check out client.js and server.js for the Y-Sweet integration.

export function createColorGridUI(colors, gridSize, onChange) {
  let curColorIdx = 0

  const onSelect = (idx) => {
    curColorIdx = idx
  }
  const colorOptionsEl = createColorOptions(colors, onSelect)

  const onCellColorChange = (key) => onChange(key, colors[curColorIdx])
  const { gridEl, cellElsMap } = createGrid(gridSize, onCellColorChange)

  const colorGridEl = document.createElement('div')
  colorGridEl.appendChild(colorOptionsEl)
  colorGridEl.appendChild(gridEl)

  function updateCell(key, color) {
    const cellEl = cellElsMap.get(key)
    cellEl.style.backgroundColor = color ?? ''
  }

  return { colorGridEl, updateCell }
}

function createColorOptions(colors, onSelect) {
  const container = document.createElement('div')
  container.classList.add('color-options')
  const colorOptionEls = colors.map((color, idx) => {
    const div = document.createElement('div')
    div.classList.add('color-option')
    div.style.backgroundColor = color
    div.addEventListener('click', () => {
      onSelect(idx)
      colorOptionEls.forEach((el) => el.classList.remove('selected'))
      div.classList.add('selected')
    })
    return div
  })

  colorOptionEls[0].classList.add('selected')
  colorOptionEls.forEach((el) => container.appendChild(el))

  return container
}

function createGrid(gridSize, onClick) {
  const cellElsMap = new Map()
  const gridEl = document.createElement('table')
  const grid = gridEl.appendChild(document.createElement('tbody'))
  Array.from({ length: gridSize }, (_, y) => {
    const tr = document.createElement('tr')
    Array.from({ length: gridSize }, (_, x) => {
      const key = `${x},${y}`
      const td = document.createElement('td')
      td.classList.add('cell')
      td.addEventListener('click', () => {
        onClick(key)
      })
      tr.appendChild(td)
      cellElsMap.set(key, td)
    })
    grid.appendChild(tr)
  })
  return { gridEl, cellElsMap }
}
