import { Debuggable, DebuggableEntry, EntityType } from '.'

export function debuggableJsValue(value: any): Debuggable {
  if (typeof value === 'object') {
    if (Array.isArray(value)) {
      return new JsList(value)
    } else {
      return new JsObject(value)
    }
  } else {
    return new Scalar(value)
  }
}

export class Scalar implements Debuggable {
  constructor(private readonly _value: any) {}

  type(): EntityType {
    return 'scalar'
  }

  entries(): DebuggableEntry[] {
    return []
  }

  value(): any {
    return this._value
  }

  size(): number {
    return 0
  }

  listen(): () => void {
    return () => {}
  }
}

export class JsList implements Debuggable {
  constructor(private readonly _value: any[]) {}

  type(): EntityType {
    return 'list'
  }

  entries(): DebuggableEntry[] {
    return this._value.map((value, index) => ({ key: index, value: debuggableJsValue(value) }))
  }

  value(): any {
    return null
  }

  size(): number {
    return this._value.length
  }

  listen(): () => void {
    return () => {}
  }
}

export class JsObject implements Debuggable {
  constructor(private readonly _value: object) {}

  type(): EntityType {
    return 'object'
  }

  entries(): DebuggableEntry[] {
    return Object.entries(this._value)
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([key, value]) => ({ key, value: debuggableJsValue(value) }))
  }

  value(): any {
    return null
  }

  size(): number {
    return Object.keys(this._value).length
  }

  listen(): () => void {
    return () => {}
  }
}
