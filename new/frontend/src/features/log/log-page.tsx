export default function LogPage() {
  return (
    <main className="min-h-0 w-full flex-1 overflow-y-auto overscroll-contain sm:[scrollbar-gutter:stable_both-edges]">
      <div className="mx-auto w-full max-w-[var(--page-max-width)] px-[var(--page-padding)] pt-[max(0.75rem,env(safe-area-inset-top,0px))] pb-[calc(var(--nav-clearance)+2.5rem)]">
        <h1 className="text-xl font-semibold tracking-tight text-gray-950">
          Log
        </h1>
      </div>
    </main>
  );
}
