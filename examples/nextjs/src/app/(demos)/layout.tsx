import CopyLink from '@/components/CopyLink'

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="h-full flex flex-col">
      <CopyLink />
      <div className="h-full relative w-auto bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-white/90 via-pink-50/90 to-pink-100/90 rounded-lg">
        {children}
      </div>
    </div>
  )
}
