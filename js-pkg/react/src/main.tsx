'use client'

import {
  YSweetProvider,
  YSweetProviderParams,
  createYjsProvider,
  debuggerUrl,
} from '@y-sweet/client'
import type { AuthEndpoint, YSweetProviderWithClientToken } from '@y-sweet/client'
import type { ReactNode } from 'react'
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { Awareness } from 'y-protocols/awareness'
import * as Y from 'yjs'
export {
  createYjsProvider,
  YSweetProvider,
  debuggerUrl,
  type YSweetProviderParams,
  type YSweetProviderWithClientToken,
  type AuthEndpoint,
}

type YjsContextType = {
  doc: Y.Doc
  provider: YSweetProviderWithClientToken
}

const YjsContext = createContext<YjsContextType | null>(null)

type YDocOptions = {
  hideDebuggerLink?: boolean
}

/**
 * React hook to get the Y.Doc instance from the current context.
 *
 * @returns The Y.Doc instance.
 */
export function useYDoc(options?: YDocOptions): Y.Doc {
  const yjsCtx = useContext(YjsContext)

  useEffect(() => {
    if (!options?.hideDebuggerLink && yjsCtx) {
      const url = debuggerUrl(yjsCtx.provider.clientToken)
      console.log(
        `%cOpen this in Y-Sweet Debugger ⮕ ${url}`,
        'font-size: 1.5em; display: block; padding: 10px;',
      )
      console.log(
        '%cTo hide the debugger link, pass { hideDebuggerLink: true } to useYDoc',
        'font-style: italic;',
      )
    }
  }, [options?.hideDebuggerLink, yjsCtx])

  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.doc
}

/**
 * React hook to get the Y.Doc instance from the current context.
 *
 * @returns A debugger URL as a string.
 */
export function useYSweetDebugUrl(): string {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return debuggerUrl(yjsCtx.provider.clientToken)
}

/**
 * React hook for obtaining the YSweetProvider instance from the current context.
 *
 * This is useful for integrating components that expect a direct reference
 * to the provider.
 *
 * @returns The YSweetProvider instance.
 */
export function useYjsProvider(): YSweetProvider {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.provider
}

/**
 * React hook for obtaining the Yjs Awareness instance from the current context.
 *
 * @returns The Yjs Awareness instance for the current document.
 */
export function useAwareness(): Awareness {
  const yjsCtx = useContext(YjsContext)
  if (!yjsCtx) {
    throw new Error('Yjs hooks must be used within a YDocProvider')
  }
  return yjsCtx.provider.awareness
}

/**
 * Options for the {@link usePresence} hook.
 */
type UsePresenceOptions = {
  /** Whether the presence object should include the local client.
   * Defaults to `false`.
   */
  includeSelf?: boolean
}

/**
 * React hook that returs a setter function for the local presence object.
 *
 * @returns A setter function for the local presence object.
 */
export function usePresenceSetter<T extends Record<string, any>>(): (presence: T) => void {
  const awareness = useAwareness()

  const setLocalPresence = useCallback(
    (localState: any) => {
      if (awareness) {
        awareness.setLocalState(localState)
      }
    },
    [awareness],
  )

  return setLocalPresence
}

/** React hook that returns other users’ presence values as a `Map<number, any>`. */
export function usePresence<T extends Record<string, any>>(
  options?: UsePresenceOptions,
): Map<number, T> {
  const awareness = useAwareness()
  const [presence, setPresence] = useState<Map<number, T>>(new Map())

  const includeSelf = options?.includeSelf || false

  useEffect(() => {
    if (awareness) {
      const callback = () => {
        const map = new Map()
        awareness.getStates().forEach((state, clientID) => {
          if (!includeSelf && clientID === awareness.clientID) return

          if (Object.keys(state).length > 0) {
            map.set(clientID, state)
          }
        })

        setPresence(map)
      }
      awareness.on('change', callback)
      return () => {
        awareness.off('change', callback)
      }
    }
  }, [awareness])

  return presence
}

/**
 * Props to the {@link YDocProvider} component.
 */
export type YDocProviderProps = {
  /** The children to render. */
  children: ReactNode

  /** Response of a `getConnectionKey` call, passed from server to client. */
  docId: string

  /** The endpoint to use for authentication. */
  authEndpoint: AuthEndpoint

  /** If set to a string, the URL query parameter with this name
   * will be set to the doc id from connectionKey. */
  setQueryParam?: string
}

/**
 * A React component that provides a Y.Doc instance to its children.
 */
export function YDocProvider(props: YDocProviderProps) {
  const { children, docId, authEndpoint } = props

  const [ctx, setCtx] = useState<YjsContextType | null>(null)

  useEffect(() => {
    let canceled = false
    let provider: YSweetProviderWithClientToken | null = null
    const doc = new Y.Doc()

    ;(async () => {
      provider = await createYjsProvider(doc, docId, authEndpoint, {
        // TODO: this disables local cross-tab communication, which makes
        // debugging easier, but should be re-enabled eventually
        disableBc: true,
      })

      if (canceled) {
        provider.destroy()
        return
      }

      setCtx({ doc, provider })
    })()

    return () => {
      canceled = true
      provider?.destroy()
      doc.destroy()
    }
  }, [docId])

  useEffect(() => {
    if (props.setQueryParam) {
      const url = new URL(window.location.href)
      url.searchParams.set(props.setQueryParam, docId)
      window.history.replaceState({}, '', url.toString())
    }
  }, [props.setQueryParam, docId])

  if (ctx === null) return null

  return <YjsContext.Provider value={ctx}>{children}</YjsContext.Provider>
}

function useRedraw() {
  const [_, setRedraw] = useState(0)
  return useCallback(() => setRedraw((x) => x + 1), [setRedraw])
}

/** Represents possible values to pass to hooks that return Yjs objects,
 * which determines whether or not they trigger a re-render when the
 * Yjs object changes.
 *
 * - `'deep'` will re-render when any nested property changes.
 * - `'shallow'` will re-render when the object itself changes.
 * - `'none'` will never re-render.
 */
export type ObserverKind = 'deep' | 'shallow' | 'none'

/**
 * Options for hooks that return Yjs objects, like {@link useMap}.
 *
 * @see {@link ObserverKind}
 */
export type ObjectOptions = {
  observe?: ObserverKind
}

/**
 * Returns a `Y.Map<T>` object from the `Y.Doc` in the current context.
 *
 * The string `name` is the name of the top-level Yjs object to return.
 * Two clients that call `useMap(...)` with the same `name` will get
 * the same object.
 *
 * By default, this will subscribe the calling component to updates on
 * the object and its children. See {@link ObjectOptions} and
 * {@link ObserverKind} for finer control of observer behavior.
 *
 * @typeParam T The type of the values in the map. Keys are always strings.
 * @param name The name of the top-level Yjs object to return.
 * @param objectOptions
 * @returns
 */
export function useMap<T>(name: string, objectOptions?: ObjectOptions): Y.Map<T> {
  const doc = useYDoc({ hideDebuggerLink: true })
  const map = useMemo(() => doc.getMap<T>(name), [doc, name])
  useObserve(map, objectOptions?.observe || 'deep')

  return map
}

/**
 * Returns a `Y.Array<T>` object from the `Y.Doc` in the current context.
 *
 * The string `name` is the name of the top-level Yjs object to return.
 * Two clients that call `useArray(...)` with the same `name` will get
 * the same object.
 *
 * By default, this will subscribe the calling component to updates on
 * the object and its children. See {@link ObjectOptions} and
 * {@link ObserverKind} for finer control of observer behavior.
 *
 * @typeParam T The type of the values in the array.
 * @param name The name of the top-level Yjs object to return.
 * @param objectOptions
 * @returns
 */
export function useArray<T>(name: string, objectOptions?: ObjectOptions): Y.Array<T> {
  const doc = useYDoc({ hideDebuggerLink: true })
  const array = useMemo(() => doc.getArray<T>(name), [doc, name])
  useObserve(array, objectOptions?.observe || 'deep')

  return array
}

/**
 * Returns a `Y.Text` object from the `Y.Doc` in the current context.
 *
 * The string `name` is the name of the top-level Yjs object to return.
 * Two clients that call `useText(...)` with the same `name` will get
 * the same object.
 *
 * By default, this will subscribe the calling component to updates on
 * the object and its children. See {@link ObjectOptions} and
 * {@link ObserverKind} for finer control of observer behavior.
 *
 * @param name The name of the top-level Yjs object to return.
 * @param objectOptions
 * @returns
 */
export function useText(name: string, observerKind?: ObjectOptions): Y.Text {
  const doc = useYDoc({ hideDebuggerLink: true })
  const text = useMemo(() => doc.getText(name), [doc, name])
  useObserve(text, observerKind?.observe || 'deep')

  return text
}

/**
 * A hook that causes its calling component to re-render when the given
 * Yjs object changes.
 *
 * The `kind` parameter determines the level of change that will trigger
 * a re-render. See {@link ObserverKind} for more information.
 *
 * @param object The Yjs object to observe.
 * @param kind The kind of observation to perform.
 */
export function useObserve(object: Y.AbstractType<any>, kind: ObserverKind) {
  const redraw = useRedraw()

  useEffect(() => {
    if (kind === 'deep') {
      object.observeDeep(redraw)
    } else if (kind === 'shallow') {
      object.observe(redraw)
    }

    return () => {
      if (kind === 'deep') {
        object.unobserveDeep(redraw)
      } else if (kind === 'shallow') {
        object.unobserve(redraw)
      }
    }
  })
}
