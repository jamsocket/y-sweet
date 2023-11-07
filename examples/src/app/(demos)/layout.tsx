import CopyLink from '@/components/CopyLink'

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <CopyLink />
      <div className="absolute w-full h-full top-0 pt-14 lg:relative lg:w-auto lg:pt-0 bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-white/90 via-pink-50/90 to-pink-100/90 sm:mr-2 sm:rounded-lg">
        {children}
      </div>
    </>
  )
}
