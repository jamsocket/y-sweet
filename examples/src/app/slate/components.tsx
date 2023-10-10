import React, { PropsWithChildren } from 'react'

function cn(...args: unknown[]) {
  return args.filter(Boolean).join(' ')
}

interface BaseProps {
  className: string
  [key: string]: unknown
}
type OrNull<T> = T | null

export const Button = React.forwardRef(
  (
    {
      className,
      active,
      ...props
    }: PropsWithChildren<
      {
        active: boolean
      } & BaseProps
    >,
    ref: React.Ref<HTMLSpanElement>,
  ) => (
    <span
      {...props}
      ref={ref}
      className={cn(
        className,
        'cursor-pointer flex justify-center items-center m-1 rounded',
        active ? 'text-white bg-pink-900' : 'text-gray-400 hover:bg-pink-100 hover:text-black',
      )}
    />
  ),
)
Button.displayName = 'Button'

export const Menu = React.forwardRef(
  ({ className, ...props }: PropsWithChildren<BaseProps>, ref: React.Ref<HTMLDivElement>) => (
    <div {...props} data-test-id="menu" ref={ref} className={cn(className, '')} />
  ),
)
Menu.displayName = 'Menu'

export const Toolbar = React.forwardRef(
  ({ className, ...props }: PropsWithChildren<BaseProps>, ref: React.Ref<HTMLDivElement>) => (
    <Menu
      {...props}
      ref={ref}
      className={cn(
        className,
        'relative border border-b border-gray-200 flex flex-row items-center',
      )}
    />
  ),
)

Toolbar.displayName = 'Toolbar'
