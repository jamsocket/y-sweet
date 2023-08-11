import * as Y from 'yjs'
import { randomColor, randomString } from './util'
import { RadixPriorityQueueBuilder } from './radixpq'

export const ROOT_ID = '__root'
const PARENT = 'parent'
const COLOR = 'color'

type NodeRelations = { parent: string | null; children: Set<string> }

function highestPriorityParent(map: Record<string, number>): [string | null, number] {
  let maxPriority = 0
  let maxParent = null
  Object.entries(map).forEach(([parent, priority]) => {
    if (priority > maxPriority) {
      maxPriority = priority
      maxParent = parent
    }
  })
  return [maxParent, maxPriority]
}

type JsonNode = {
  parent: {
    [key: string]: number
  }
  [key: string]: any
}

type JsonMap = {
  [key: string]: JsonNode
}

export function buildTree(map: JsonMap): [Map<string, NodeRelations>, number] {
  let maxClock = 0

  // First, create a map of all nodes to the potential children that have them as a top-priority parent.
  let unrootedNodes = new Map<string, Set<string>>()

  Object.entries(map).forEach(([id, node]) => {
    let [parent, priority] = highestPriorityParent(node[PARENT])
    if (!parent) {
      console.warn(`Ignoring node ${id} which has no parent.`, node)
      return
    }
    if (!unrootedNodes.has(parent)) {
      unrootedNodes.set(parent, new Set<string>())
    }
    unrootedNodes.get(parent)!.add(id)
    maxClock = Math.max(maxClock, priority)
  })

  // Then, recurse from the root and build the tree.
  let rootedNodes = new Map<string, NodeRelations>()

  function recursivelyParent(id: string) {
    let children = unrootedNodes.get(id)
    if (!children) {
      return // node has been parented
    }

    for (let child of children) {
      if (rootedNodes.has(child)) {
        continue
      }
      rootedNodes.set(child, { parent: id, children: new Set<string>() })
      rootedNodes.get(id)!.children.add(child)
      recursivelyParent(child)
    }

    unrootedNodes.delete(id)
  }

  rootedNodes.set(ROOT_ID, { parent: null, children: new Set<string>() })
  recursivelyParent(ROOT_ID)

  // Now, parent the remaining nodes by breaking ties.

  let queueBuilder = new RadixPriorityQueueBuilder<[string, string]>() // [child, parent]

  unrootedNodes.forEach((_, nodeId) => {
    let node = map[nodeId]!
    let parents: Record<string, number> = node[PARENT]
    for (let [parent, priority] of Object.entries(parents)) {
      queueBuilder.addEntry(priority, [nodeId, parent])
    }
  })

  let nodeQueue = queueBuilder.build()

  for (let [child, parent] of nodeQueue) {
    if (rootedNodes.has(parent) && !rootedNodes.has(child)) {
      // node's parent has been parented, but node hasn't
      rootedNodes.get(parent)!.children.add(child)
      rootedNodes.set(child, { parent: parent, children: new Set<string>() })
      recursivelyParent(child)
    }
  }

  if (unrootedNodes.size > 0) {
    console.warn('Some nodes left unrooted!', unrootedNodes)
  }

  return [rootedNodes, maxClock]
}

export class YTree {
  // Map of parent id to map of child YTreeNodes. A null parent id means the root.
  structure: Map<string, NodeRelations> = new Map()
  maxClock: number = 0
  onChange?: () => void = () => {}

  constructor(public map: Y.Map<Y.Map<any>>) {
    this.map.observeDeep((e) => {
      this.updateChildren()
    })
    this.updateChildren()
  }

  setOnChange(onChange?: () => void) {
    this.onChange = onChange
  }

  root() {
    return new YTreeNode(ROOT_ID, this)
  }

  updateChildren() {
    let map = this.map.toJSON()

    let [structure, maxClock] = buildTree(map)
    this.maxClock = maxClock

    this.structure = structure
    if (this.onChange) {
      this.onChange()
    }
  }
}

export class YTreeNode {
  constructor(
    private _id: string,
    public tree: YTree,
  ) {}

  id(): string {
    return this._id
  }

  color(): string {
    if (this._id === ROOT_ID) {
      return '#F2FF8C'
    } else {
      return this.tree.map.get(this._id)!.get(COLOR)
    }
  }

  children(): YTreeNode[] {
    return Array.from(this.tree.structure.get(this._id)?.children || []).map(
      (id) => new YTreeNode(id, this.tree),
    )
  }

  addChild(): YTreeNode {
    let map = new Y.Map()
    const id = randomString()
    const color = randomColor()
    let parentMap = new Y.Map()
    parentMap.set(this._id, ++this.tree.maxClock)
    map.set(PARENT, parentMap)
    map.set(COLOR, color)
    this.tree.map.set(id, map)
    return new YTreeNode(id, this.tree)
  }

  reparent(newParent: YTreeNode) {
    if (this._id === ROOT_ID) {
      console.error("Can't reparent root.")
      return
    }

    this.tree.map.doc!.transact(() => {
      let oldParent = this.tree.structure.get(this._id)!.parent!

      if (newParent.id() === this._id) {
        return
      }

      // Detect if the new parent was a descendant of the new child.
      let probe = newParent.id()
      while (probe !== ROOT_ID) {
        let probeParent = this.tree.structure.get(probe)!.parent!
        if (probeParent === this._id) {
          // If the new parent was a descendant of the new child, the old node has a
          // child node (probe) which is an ancestor of the new parent. We promote that child
          // to the parent's place by reparenting it to the original parent's parent.
          this.tree.map.get(probe)!.get(PARENT).set(oldParent, ++this.tree.maxClock)
          break
        }
        probe = probeParent
      }

      this.tree.map.get(this._id)!.get(PARENT).set(newParent.id(), ++this.tree.maxClock)
    })
  }
}
