import React, { useCallback, useMemo, useEffect } from 'react'
import { flushSync } from 'react-dom'
import isHotkey from 'is-hotkey'
import { Editable, withReact, useSlate, Slate } from 'slate-react'
import type { ReactEditor, RenderElementProps, RenderLeafProps } from 'slate-react'
import { Editor, Transforms, createEditor, Element as SlateElement, Text } from 'slate'
import type { BaseEditor, Descendant } from 'slate'
import * as Y from 'yjs'
import { withYjs, YjsEditor, withCursors, withYHistory } from '@slate-yjs/core'
import type { Awareness } from 'y-protocols/awareness'
import {
  getRemoteCaretsOnLeaf,
  getRemoteCursorsOnLeaf,
  useDecorateRemoteCursors,
} from '@slate-yjs/react'

import { Button, Toolbar } from './components'

import {
  LuBold,
  LuItalic,
  LuCode,
  LuUnderline,
  LuHeading1,
  LuHeading2,
  LuTextQuote,
  LuList,
  LuListOrdered,
  LuAlignCenter,
  LuAlignLeft,
  LuAlignRight,
  LuAlignJustify,
} from 'react-icons/lu'

const ParagraphType = 'paragraph'
const HeadingOneType = 'heading-one'
const HeadingTwoType = 'heading-two'
const BlockquoteType = 'block-quote'
const BulletedListType = 'bulleted-list'
const NumberedListType = 'numbered-list'
const ListItemType = 'list-item'
const AlignLeft = 'left'
const AlignCenter = 'center'
const AlignRight = 'right'
const AlignJustify = 'justify'
const BoldMark = 'bold'
const ItalicMark = 'italic'
const UnderlineMark = 'underline'
const CodeMark = 'code'

type BlockType =
  | typeof ParagraphType
  | typeof HeadingOneType
  | typeof HeadingTwoType
  | typeof BlockquoteType
  | typeof BulletedListType
  | typeof NumberedListType
  | typeof ListItemType
type AlignType = typeof AlignLeft | typeof AlignCenter | typeof AlignRight | typeof AlignJustify
type MarkType = typeof BoldMark | typeof ItalicMark | typeof UnderlineMark | typeof CodeMark

type ParagraphElement = { type: typeof ParagraphType; children: Descendant[] }
type HeadingOneElement = { type: typeof HeadingOneType; children: Descendant[] }
type HeadingTwoElement = { type: typeof HeadingTwoType; children: Descendant[] }
type BlockquoteElement = { type: typeof BlockquoteType; children: Descendant[] }
type BulletedListElement = { type: typeof BulletedListType; children: Descendant[] }
type NumberedListElement = { type: typeof NumberedListType; children: Descendant[] }
type ListItemElement = { type: typeof ListItemType; children: Descendant[] }
type AlignLeftElement = { align: typeof AlignLeft; children: Descendant[] }
type AlignCenterElement = { align: typeof AlignCenter; children: Descendant[] }
type AlignRightElement = { align: typeof AlignRight; children: Descendant[] }
type AlignJustifyElement = { align: typeof AlignJustify; children: Descendant[] }
type BoldText = { [BoldMark]?: true }
type ItalicText = { [ItalicMark]?: true }
type UnderlineText = { [UnderlineMark]?: true }
type CodeText = { [CodeMark]?: true }

type CustomElement =
  | ParagraphElement
  | HeadingOneElement
  | HeadingTwoElement
  | BlockquoteElement
  | BulletedListElement
  | NumberedListElement
  | ListItemElement
  | AlignLeftElement
  | AlignCenterElement
  | AlignRightElement
  | AlignJustifyElement
type CustomText = { text: string } & BoldText & ItalicText & UnderlineText & CodeText

declare module 'slate' {
  interface CustomTypes {
    Editor: BaseEditor & ReactEditor
    Element: CustomElement
    Text: CustomText
  }
}

const HOTKEYS: Record<string, MarkType> = {
  'mod+b': BoldMark,
  'mod+i': ItalicMark,
  'mod+u': UnderlineMark,
  'mod+`': CodeMark,
}

const LIST_TYPES = [NumberedListType, BulletedListType]
const TEXT_ALIGN_TYPES = [AlignLeft, AlignCenter, AlignRight, AlignJustify]

function DecoratedEditable(props: React.ComponentProps<typeof Editable>) {
  const decorate = useDecorateRemoteCursors()

  return <Editable {...props} decorate={decorate} renderLeaf={renderDecoratedLeaf} />
}

const RichtextSlateEditor = ({
  sharedType,
  awareness,
}: {
  sharedType: Y.XmlText
  awareness: Awareness
}) => {
  const renderElement = useCallback((props: RenderElementProps) => <Element {...props} />, [])
  const editor = useMemo(() => {
    const e = withReact(
      withYHistory(
        withCursors(withYjs(createEditor(), sharedType), awareness, {
          data: {
            color: `#${Math.floor(Math.random() * 16777215).toString(16)}`,
            name: 'Anonymous Coward',
          },
        }),
      ),
    )

    const { normalizeNode } = e
    e.normalizeNode = (entry) => {
      const [node] = entry
      if (!Editor.isEditor(node) || node.children.length > 0) {
        return normalizeNode(entry)
      }

      Transforms.insertNodes(e, initialValue, { at: [0] })
    }

    return e
  }, [sharedType, awareness])

  useEffect(() => {
    YjsEditor.connect(editor)
    return () => YjsEditor.disconnect(editor)
  }, [editor])

  return (
    <Slate editor={editor} initialValue={initialValue}>
      <Toolbar>
        <MarkButton format="bold" icon={<LuBold />} />
        <MarkButton format="italic" icon={<LuItalic />} />
        <MarkButton format="underline" icon={<LuUnderline />} />
        <MarkButton format="code" icon={<LuCode />} />
        <BlockButton format="heading-one" icon={<LuHeading1 />} />
        <BlockButton format="heading-two" icon={<LuHeading2 />} />
        <BlockButton format="block-quote" icon={<LuTextQuote />} />
        <BlockButton format="numbered-list" icon={<LuListOrdered />} />
        <BlockButton format="bulleted-list" icon={<LuList />} />
        <BlockButton format="left" icon={<LuAlignLeft />} />
        <BlockButton format="center" icon={<LuAlignCenter />} />
        <BlockButton format="right" icon={<LuAlignRight />} />
        <BlockButton format="justify" icon={<LuAlignJustify />} />
      </Toolbar>
      <div className="p-2">
        <DecoratedEditable
          renderElement={renderElement}
          className="p-2 min-h-[100px] !outline-none"
          spellCheck
          autoFocus
          onKeyDown={(event) => {
            for (const hotkey in HOTKEYS) {
              if (isHotkey(hotkey, event as any)) {
                event.preventDefault()

                const mark = HOTKEYS[hotkey]
                toggleMark(editor, mark)
              }
            }
          }}
        />
      </div>
    </Slate>
  )
}

const toggleBlock = (editor: Editor, format: BlockType | AlignType) => {
  const isActive = isBlockActive(
    editor,
    format,
    TEXT_ALIGN_TYPES.includes(format) ? 'align' : 'type',
  )
  const isList = LIST_TYPES.includes(format)

  Transforms.unwrapNodes(editor, {
    match: (n) =>
      !Editor.isEditor(n) &&
      SlateElement.isElement(n) &&
      'type' in n &&
      LIST_TYPES.includes(n.type) &&
      !TEXT_ALIGN_TYPES.includes(format),
    split: true,
  })
  let newProperties: Partial<SlateElement>
  if (TEXT_ALIGN_TYPES.includes(format)) {
    newProperties = {
      align: isActive ? undefined : (format as AlignType),
    }
  } else {
    newProperties = {
      type: isActive ? 'paragraph' : isList ? 'list-item' : (format as BlockType),
    }
  }
  Transforms.setNodes<SlateElement>(editor, newProperties)

  if (!isActive && isList) {
    const block = { type: format, children: [] } as BulletedListElement | NumberedListElement
    Transforms.wrapNodes(editor, block)
  }
}

const toggleMark = (editor: Editor, format: MarkType) => {
  const isActive = isMarkActive(editor, format)

  if (isActive) {
    Editor.removeMark(editor, format)
  } else {
    Editor.addMark(editor, format, true)
  }
}

const isBlockActive = (editor: Editor, format: string, blockType: 'type' | 'align' = 'type') => {
  const { selection } = editor
  if (!selection) return false

  const [match] = Array.from(
    Editor.nodes(editor, {
      at: Editor.unhangRange(editor, selection),
      match: (n) =>
        !Editor.isEditor(n) && SlateElement.isElement(n) && (n as any)[blockType] === format,
    }),
  )

  return !!match
}

const isMarkActive = (editor: Editor, format: MarkType) => {
  const marks = Editor.marks(editor)
  return marks ? marks[format] === true : false
}

const Element = ({ attributes, children, element }: RenderElementProps) => {
  const style = { textAlign: 'align' in element ? element.align : undefined }

  if ('type' in element) {
    switch (element.type) {
      case 'block-quote':
        return (
          <blockquote
            className="pl-2 mb-2 border-l-2 border-gray-400"
            style={style}
            {...attributes}
          >
            {children}
          </blockquote>
        )
      case 'bulleted-list':
        return (
          <ul className="list-disc list-inside" style={style} {...attributes}>
            {children}
          </ul>
        )
      case 'heading-one':
        return (
          <h1 className="pb-2 text-xl font-bold" style={style} {...attributes}>
            {children}
          </h1>
        )
      case 'heading-two':
        return (
          <h2 className="pb-2 text-lg" style={style} {...attributes}>
            {children}
          </h2>
        )
      case 'list-item':
        return (
          <li style={style} {...attributes}>
            {children}
          </li>
        )
      case 'numbered-list':
        return (
          <ol className="list-decimal list-inside" style={style} {...attributes}>
            {children}
          </ol>
        )
      default:
        return (
          <p className="mb-2" style={style} {...attributes}>
            {children}
          </p>
        )
    }
  }
}

const Leaf = ({ attributes, children, leaf }: RenderLeafProps) => {
  if ('bold' in leaf && leaf.bold) {
    children = <strong>{children}</strong>
  }

  if ('code' in leaf && leaf.code) {
    children = <code>{children}</code>
  }

  if ('italic' in leaf && leaf.italic) {
    children = <em>{children}</em>
  }

  if ('underline' in leaf && leaf.underline) {
    children = <u>{children}</u>
  }

  return <span {...attributes}>{children}</span>
}

type CursorData = {
  name: string
  color: string
}

function addAlpha(hexColor: string, opacity: number): string {
  const normalized = Math.round(Math.min(Math.max(opacity, 0), 1) * 255)
  return hexColor + normalized.toString(16).toUpperCase()
}

function renderDecoratedLeaf(props: RenderLeafProps) {
  getRemoteCursorsOnLeaf<CursorData, Text>(props.leaf).forEach((cursor) => {
    if (cursor.data) {
      props.children = (
        <span style={{ backgroundColor: addAlpha(cursor.data.color, 0.5) }}>{props.children}</span>
      )
    }
  })

  getRemoteCaretsOnLeaf<CursorData, Text>(props.leaf).forEach((caret) => {
    if (caret.data) {
      props.children = (
        <span className="relative">
          <span
            contentEditable={false}
            className="absolute top-0 bottom-0 w-0.5 left-[-1px]"
            style={{ backgroundColor: caret.data.color }}
          />
          <span
            contentEditable={false}
            className="absolute text-xs text-white left-[-1px] top-0 whitespace-nowrap rounded rounded-bl-none px-1.5 py-0.5 select-none"
            style={{
              backgroundColor: caret.data.color,
              transform: 'translateY(-100%)',
            }}
          >
            {caret.data.name}
          </span>
          {props.children}
        </span>
      )
    }
  })

  return <Leaf {...props} />
}

const BlockButton = ({
  format,
  icon,
}: {
  format: BlockType | AlignType
  icon: React.ReactNode
}) => {
  const editor = useSlate()
  return (
    <Button
      className="w-8 h-8"
      active={isBlockActive(editor, format, TEXT_ALIGN_TYPES.includes(format) ? 'align' : 'type')}
      onMouseDown={(event: Event) => {
        event.preventDefault()
        toggleBlock(editor, format)
      }}
    >
      {icon}
    </Button>
  )
}

const MarkButton = ({ format, icon }: { format: MarkType; icon: React.ReactNode }) => {
  const editor = useSlate()
  return (
    <Button
      className="w-8 h-8"
      active={isMarkActive(editor, format)}
      onMouseDown={(event: Event) => {
        event.preventDefault()
        toggleMark(editor, format)
      }}
    >
      {icon}
    </Button>
  )
}

const initialValue: Descendant[] = [
  {
    type: 'paragraph',
    children: [{ text: '' }],
  },
]

export default RichtextSlateEditor
