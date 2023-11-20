import { YTreeNode } from './ytree'

export type Point = {
  x: number
  y: number
  radius?: number
}

export interface LayoutNode extends Point {
  parent: LayoutNode | null
  treeNode: YTreeNode
}

export class NodeBox {
  private width: number
  private height: number
  private children: NodeBox[]

  constructor(private treeNode: YTreeNode) {
    this.children = treeNode.children().map((child) => new NodeBox(child))
    this.width = Math.max(
      1,
      this.children.reduce((acc, child) => acc + child.width, 0),
    )
    this.height = this.children.reduce((acc, child) => Math.max(acc, child.height), 0) + 1
  }

  private getChildrenInner(
    xOffset: number,
    yOffset: number,
    xSize: number,
    ySize: number,
    parent: LayoutNode | null,
  ): LayoutNode[] {
    let nodes: LayoutNode[] = []
    const thisNode: LayoutNode = {
      x: (xOffset + this.width / 2) * xSize,
      y: yOffset * ySize + ySize / 2,
      parent,
      radius: 20,
      treeNode: this.treeNode,
    }
    nodes.push(thisNode)

    for (let child of this.children) {
      nodes.push(...child.getChildrenInner(xOffset, yOffset + 1, xSize, ySize, thisNode))
      xOffset += child.width
    }

    return nodes
  }

  getChildren(): LayoutNode[] {
    return this.getChildrenInner(0, 0, 1 / this.width, 1 / this.height, null).sort((a, b) =>
      a.treeNode.id().localeCompare(b.treeNode.id()),
    )
  }
}
