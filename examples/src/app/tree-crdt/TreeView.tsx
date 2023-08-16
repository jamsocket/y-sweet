'use client'

import { useEffect, useMemo, useState } from 'react'
import { LayoutNode, NodeBox, Point } from './tree_layout'
import { ROOT_ID, YTree, YTreeNode } from './ytree'
import { useMap } from '@y-sweet/react'
import * as Y from 'yjs'
import CopyLink from '@/components/CopyLink'
import Paragraph from '@/components/Paragraph'
import { Link } from '@/components/Link'

function useDragHelper(callback: (origin: LayoutNode, target: LayoutNode) => void) {
  const [dragOrigin, setDragOrigin] = useState<LayoutNode | null>(null)
  const [dragTarget, setDragTarget] = useState<LayoutNode | null>(null)

  return {
    dragOrigin,
    dragTarget,
    mouseOver: (node: LayoutNode | null) => {
      if (dragOrigin !== null) {
        setDragTarget(node)
      } else if (node !== null) {
        setDragOrigin(node)
      }
    },
    mouseUp: () => {
      if (dragOrigin !== null && dragTarget !== null && dragOrigin !== dragTarget) {
        callback(dragOrigin, dragTarget)
      }
      setDragOrigin(null)
      setDragTarget(null)
    },
  }
}

function Arrow(props: { origin: Point; target: Point; offset?: number; animate: boolean }) {
  let length = Math.sqrt(
    (props.target.x - props.origin.x) ** 2 + (props.target.y - props.origin.y) ** 2,
  )

  if (length < (props.target.radius || 0)) {
    return null
  }

  let shrunkLength = 1 - (length - (props.target.radius || 0) - 2) / length

  let target = {
    x: props.target.x - (props.target.x - props.origin.x) * shrunkLength,
    y: props.target.y - (props.target.y - props.origin.y) * shrunkLength,
  }
  let origin = props.origin

  let path = `M ${origin.x} ${origin.y} L ${target.x} ${target.y}`

  let vector = [(origin.x - target.x) / length, (origin.y - target.y) / length]
  let perp = [-vector[1], vector[0]]

  let arrowLength = 10
  let arrowWidth = 5

  let points = [
    [target.x, target.y],
    [
      target.x + arrowLength * vector[0] + arrowWidth * perp[0],
      target.y + arrowLength * vector[1] + arrowWidth * perp[1],
    ],
    [
      target.x + arrowLength * vector[0] - arrowWidth * perp[0],
      target.y + arrowLength * vector[1] - arrowWidth * perp[1],
    ],
  ]

  let arrowPath = `M ${points[0][0]} ${points[0][1]} L ${points[1][0]} ${points[1][1]} L ${points[2][0]} ${points[2][1]} Z`

  return (
    <g>
      <path
        d={path + ' ' + arrowPath}
        stroke="black"
        fill="black"
        strokeWidth={2}
        style={{
          transition: props.animate ? 'd 0.2s' : 'none',
          animation: props.animate ? 'fadeInAnimation ease 0.2s' : 'none',
        }}
      />
    </g>
  )
}

function Tree(props: { root: YTreeNode }) {
  const { dragOrigin, dragTarget, mouseOver, mouseUp } = useDragHelper(
    (origin: LayoutNode, target: LayoutNode) => {
      origin.treeNode.reparent(target.treeNode)
    },
  )

  const [mousePosition, setMousePosition] = useState<Point | null>(null)

  let nodes = useMemo(
    () => new NodeBox(props.root).getChildren(),
    [props.root, props.root.tree.structure],
  )
  const height = 600
  const width = 600
  const xScale = (x: number) => x * width
  const yScale = (y: number) => y * height

  const scalePoint = (point: Point) => ({
    x: xScale(point.x),
    y: yScale(point.y),
    radius: point.radius,
  })

  let tentativeEdgePath: [Point, Point] | null = null
  if (dragOrigin !== null) {
    if (dragTarget !== null) {
      tentativeEdgePath = [scalePoint(dragOrigin), scalePoint(dragTarget)]
    } else {
      if (mousePosition && dragOrigin) {
        tentativeEdgePath = [scalePoint(dragOrigin), mousePosition]
      }
    }
  }

  return (
    <svg
      style={{ width, height }}
      onMouseUp={mouseUp}
      onMouseMove={(e) => {
        if (e.buttons === 1) {
          let rect = e.currentTarget.getBoundingClientRect()
          let x = e.clientX - rect.left
          let y = e.clientY - rect.top

          setMousePosition({ x, y })
        }
      }}
    >
      {nodes.map(
        (node) =>
          node.parent && (
            <Arrow
              key={node.treeNode.id()}
              origin={scalePoint(node)}
              target={scalePoint(node.parent)}
              animate={true}
            />
          ),
      )}

      {tentativeEdgePath && (
        <Arrow origin={tentativeEdgePath[0]} target={tentativeEdgePath[1]} animate={false} />
      )}

      {nodes.map((node) => (
        <g
          className="select-none"
          key={node.treeNode.id()}
          style={{
            transform: `translate(${xScale(node.x)}px, ${yScale(node.y)}px)`,
            transition: 'transform 0.2s',
            animation: 'fadeInAnimation ease 0.2s',
          }}
          onMouseMove={(e) => {
            if (e.buttons === 1) {
              mouseOver(node)
            }
          }}
          onMouseOut={() => mouseOver(null)}
          onDoubleClick={() => {
            node.treeNode.addChild()
          }}
        >
          <circle
            r={node.radius}
            fill={node.treeNode.color()}
            stroke={node === dragOrigin || node === dragTarget ? 'white' : 'black'}
            strokeWidth={2}
          />
          <text
            textAnchor="middle"
            dominantBaseline="middle"
            style={{ fontSize: 12, userSelect: 'none', fontFamily: 'monospace' }}
          >
            {node.treeNode.id() === ROOT_ID ? 'root' : node.treeNode.id()}
          </text>
        </g>
      ))}
    </svg>
  )
}

export function TreeView() {
  const [tree, setTree] = useState<YTreeNode | null>(null)
  const [_, bumpTreeVersion] = useState(0)
  const treeMap = useMap<Y.Map<any>>('tree')

  useEffect(() => {
    let tree = new YTree(treeMap)
    tree.setOnChange(() => {
      bumpTreeVersion((v) => v + 1)
    })
    let root = tree.root()

    setTree(root)

    return () => {
      tree.setOnChange(undefined)
    }
  }, [treeMap])

  return (
    <div className="p-4 sm:p-8 space-y-3">
      <Paragraph>
        This is a variation of Evan Wallace’s{' '}
        <Link href="https://madebyevan.com/algos/crdt-mutable-tree-hierarchy/">
          Mutable Tree Hierarchy CRDT
        </Link>{' '}
        on top of Yjs. Double-click a node to add a child; drag a node to another node to reparent
        it.
      </Paragraph>
      {tree && <Tree root={tree} />}
      <CopyLink />
    </div>
  )
}
