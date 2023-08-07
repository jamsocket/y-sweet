import * as Y from 'yjs'
import { randomColor, randomString } from './util'
import { RadixPriorityQueueBuilder } from './radixpq'
import { ParentChain } from './parent_chain'

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

  setOnChange(onChange: () => void) {
    this.onChange = onChange
  }

  root() {
    return new YTreeNode(ROOT_ID, this)
  }

  updateChildren() {
    let map = this.map.toJSON()
    console.log('mapp...', map)

    // First, parent any child that can be parented via the most recent entry.
    // This should usually cover the vast majority of nodes.

    // Map of node IDs to their parent and children.
    let rootedNodes = new Map<string, NodeRelations>()

    // The root node is special; it cannot be reparented.
    rootedNodes.set(ROOT_ID, { parent: null, children: new Set<string>() })

    // A map of node IDs that are detached from the root to their children.
    let unrootedNodes = new Map<string, ParentChain>()

    Object.entries(map).forEach(([id, node]) => {
      // Chain of unparented parents from the node we started at.
      // Only unparented parents are added to this list; as soon as we
      // find a parented node we parent the nodes to that parent in
      // the reverse order of this list.
      let parentChain = new ParentChain()

      while (!parentChain.hasCycle()) {
        if (unrootedNodes.has(id)) {
          return
        }

        parentChain.push(id)

        if (rootedNodes.has(id)) {
          // The parent is part of a chain to the root; go down the chain and parent.

          for (let [child, parent] of parentChain.childParentPairs()) {
            rootedNodes.get(parent)!.children.add(child)
            rootedNodes.set(child, { parent: id, children: new Set<string>() })
          }

          return
        }

        let [parent, priority] = highestPriorityParent(node[PARENT])
        this.maxClock = Math.max(this.maxClock, priority)

        if (!parent) {
          console.warn(`Ignoring node ${id} which has no parent.`, node)
          return
        }

        let tryNode: Y.Map<any> | undefined
        if (parent !== ROOT_ID) {
          tryNode = map[parent]
          if (!tryNode) {
            console.warn(`Ignoring node ${parent} which does not exist.`)
            return
          }
        }

        id = parent
        node = tryNode!
      }

      console.log('unrooted', id, node[PARENT])
      for (let node of parentChain) {
        unrootedNodes.set(node, parentChain)
      }
    })

    // Now, parent the cycles.

    // let queueBuilder = new RadixPriorityQueueBuilder<[string, string]>() // [child, parent]

    // unrootedNodes.forEach(([nodeId, _]) => {
    //     console.log('unrooted', nodeId)
    //     let node = this.map.get(nodeId)!
    //     let parents = node.get(PARENT)
    //     for (let [parent, priority] of parents.entries()) {
    //         console.log('==p', parent, priority)
    //         queueBuilder.addEntry(priority, [nodeId, parent])
    //     }
    // })

    // let queue = queueBuilder.build()

    // for (let [child, parent] of queue) {
    //     console.log('pp', child, parent)
    //     let parentChain = unrootedNodes.get(child)
    //     if (!parentChain) {
    //         // node has been parented
    //         console.log('node has been parented')
    //         continue
    //     }
    //     if (rootedNodes.has(parent)) {
    //         console.log('herep')
    //         // node's parent has been parented, but node hasn't
    //         rootedNodes.get(parent)!.children.add(child)
    //         rootedNodes.set(child, { parent: parent, children: new Set<string>() })
    //         unrootedNodes.delete(child)

    //         // loop over children of node and parent them
    //         for (let [loopChild, loopParent] of parentChain.childParentPairsFrom(child)) {
    //             console.log('here1', loopChild, loopParent)
    //             if (rootedNodes.has(loopChild)) {
    //                 break // if this node is rooted, its children are too
    //             }

    //             rootedNodes.get(loopParent)!.children.add(loopChild)
    //             rootedNodes.set(loopChild, { parent: loopParent, children: new Set<string>() })
    //             unrootedNodes.delete(loopChild)
    //         }
    //         console.log('here2')
    //     }
    //     console.log('here3')
    // }

    this.structure = rootedNodes
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
    if (newParent.id() === this._id) {
      return
    }
    this.tree.map.get(this._id)!.get(PARENT).set(newParent.id(), ++this.tree.maxClock)
  }
}
