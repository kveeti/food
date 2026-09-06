export default function SignInPage() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-[var(--page-width)] items-center px-4 py-10">
      <div>
        <h1 className="text-[1.5rem] font-semibold tracking-tight text-gray-950">
          Sign in to Food
        </h1>
        <p className="mt-6">
          <a
            href="/auth/sign-in"
            className="inline-flex rounded-lg bg-gray-900 px-4 py-2.5 text-sm font-medium text-gray-0 outline-2 outline-offset-2 outline-transparent hover:bg-gray-700 focus-visible:outline-gray-500"
          >
            Sign in
          </a>
        </p>
      </div>
    </main>
  );
}
