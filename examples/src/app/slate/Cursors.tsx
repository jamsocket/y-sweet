import { CursorOverlayData, useRemoteCursorOverlayPositions } from '@slate-yjs/react'
import React, { CSSProperties, PropsWithChildren, useRef } from 'react'

type CaretProps = Pick<CursorOverlayData<CursorData>, 'caretPosition' | 'data'>

type CursorData = {
  name: string
  color: string
}

function addAlpha(hexColor: string, opacity: number): string {
  const normalized = Math.round(Math.min(Math.max(opacity, 0), 1) * 255)
  return hexColor + normalized.toString(16).toUpperCase()
}

function Caret({ caretPosition, data }: CaretProps) {
  const caretStyle: CSSProperties = {
    ...caretPosition,
    background: data?.color,
  }

  const labelStyle: CSSProperties = {
    transform: 'translateY(-100%)',
    background: data?.color,
  }

  return (
    <div style={caretStyle} className="w-0.5 absolute">
      <div
        className="absolute text-xs text-white whitespace-nowrap top-0 rounded rounded-bl-none px-1.5 py-0.5"
        style={labelStyle}
      >
        {data?.name}
      </div>
    </div>
  )
}

function RemoteSelection({ data, selectionRects, caretPosition }: CursorOverlayData<CursorData>) {
  if (!data) {
    return null
  }

  const selectionStyle: CSSProperties = {
    backgroundColor: addAlpha(data.color, 0.5),
  }

  return (
    <React.Fragment>
      {selectionRects.map((position, i) => (
        <div
          style={{ ...selectionStyle, ...position }}
          className="absolute pointer-events-none"
          key={i}
        />
      ))}
      {caretPosition && <Caret caretPosition={caretPosition} data={data} />}
    </React.Fragment>
  )
}

type RemoteCursorsProps = PropsWithChildren<{
  className?: string
}>

export function RemoteCursorOverlay({ className, children }: RemoteCursorsProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [cursors] = useRemoteCursorOverlayPositions<CursorData>({
    containerRef,
  })

  return (
    <div className={'relative ' + className} ref={containerRef}>
      {children}
      {cursors.map((cursor) => (
        <RemoteSelection key={cursor.clientId} {...cursor} />
      ))}
    </div>
  )
}
