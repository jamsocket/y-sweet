<img src="https://raw.githubusercontent.com/drifting-in-space/y-sweet/main/logo.svg" />

# @y-sweet/react

[![GitHub Repo stars](https://img.shields.io/github/stars/drifting-in-space/y-sweet?style=social)](https://github.com/drifting-in-space/y-sweet)
[![Chat on Discord](https://img.shields.io/static/v1?label=chat&message=discord&color=404eed)](https://discord.gg/N5sEpsuhh9)
[![npm](https://img.shields.io/npm/v/@y-sweet/react)](https://www.npmjs.com/package/@y-sweet/react)

React library for building collaboration features with y-sweet.

[![Version](https://img.shields.io/npm/v/@y-sweet/react)](https://npmjs.org/package/@y-sweet/react)

## Installation
```
npm install @y-sweet/react
```

## Documentation
Read the [documentation](https://www.y-sweet.dev/) for guides and API references.

## Examples
Explore our [collaborative examples](https://github.com/drifting-in-space/y-sweet) to help you get started.

All examples are open source and live in this repository, within [/examples](https://github.com/drifting-in-space/y-sweet/tree/main/examples).

# Using @y-sweet/react

### `useMap` is a shared object or map

`useMap` exposes a Yjs `Y.Map`. You can refer to [the documentation for `Y.Map` here](https://docs.yjs.dev/api/shared-types/y.map).

Our [Color Grid](/demos/color-grid) example uses `useMap` to store a grid of colors using coordinates as keys.

``` tsx filename="ColorGrid.tsx"
const items = useMap<string>('colorgrid')

items.delete(key)
items.set(`${x},${y}`, color)
```

### `useArray` is a shared array

`useArray` exposes a Yjs `Y.Array`. You can refer to [the documentation for `Y.Array` here](https://docs.yjs.dev/api/shared-types/y.array).

Our [To Do List](/demos/to-do-list) example, which uses `useArray` to store a list of To Do items.

The To Do List stores an array of objects of type Y.Map (a Yjs type). This maintains the order of the To Do List, while being able to indicate whether an item is ‘done’.

``` tsx filename="ToDoList.tsx"
const items = useArray<Y.Map<any>>('todolist')

const pushItem = useCallback((text: string) => {
    let item = new Y.Map([
        ['text', text],
        ['done', false],
    ] as [string, any][])

    items?.push([item])
},[items])

const clearCompleted = useCallback(() => {
    let indexOffset = 0
    items?.forEach((item, index) => {
        if (item.get('done')) {
            items.delete(index - indexOffset, 1)
            indexOffset += 1
        }
    })
}, [items])
```

### `useText` is a shared string

`useText` exposes a Yjs `Y.Text`. You can refer to [the documentation for `Y.Text` here](https://docs.yjs.dev/api/shared-types/y.text).

Our [Text Editor](/demos/text-editor) example, which uses `useText` to store the text document.

```tsx TextEditor.tsx
const yText = useText('text')
```

## Add presence features

### `useAwareness` returns an Yjs awareness object

The awareness object can be applied to editor bindings with Yjs that have built in presence features.

Our [Code Editor](/demos/code-editor) demo passes the awareness object to the CodeMirror Yjs binding.

```tsx CodeEditor.tsx
const awareness = useAwareness()

bindingRef.current = new CodemirrorBinding(yText!, editorRef.current, awareness)
```

### `usePresence` for general purpose presence features

The `usePresence` hook is a more opinionated service that lets you define what presence data you're saving.

This is easier for building your own live cursors or avatars than the using the `useAwareness` hook.

Our [Presence](/demos/presence) demo defines presence as a map of x,y keys and color value pairs.

```tsx Presence.tsx
const [presence, setPresence] = usePresence<{ x: number; y: number; color: string }>()

const updatePresence = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
setPresence({
    x: e.clientX - e.currentTarget.offsetLeft,
    y: e.clientY - e.currentTarget.offsetTop,
    color: myColor.current,
})
}, [setPresence])

<div
    className="border-blue-400 border relative overflow-hidden w-[500px] h-[500px]"
    onMouseMove={updatePresence}
>
    {Array.from(presence.entries()).map(([key, value]) => (
        <div
            key={key}
            className={`absolute rounded-full ${value.color}`}
            style={{ left: value.x - 6, top: value.y - 8, width: 10, height: 10 }}
        />
    ))}
</div>
```
