export function GoalSettingsSkeleton() {
  return (
    <div
      role="status"
      aria-label="Loading goals"
      className="mt-5 space-y-6 motion-safe:animate-pulse"
    >
      <div aria-hidden="true">
        <div className="mb-3 flex h-lh items-center">
          <div className="h-5 w-20 rounded bg-gray-200" />
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <GoalFieldSkeleton />
          <GoalFieldSkeleton helper />
        </div>
        <div className="mt-3 flex h-5 items-center">
          <div className="h-4 w-32 rounded bg-gray-200" />
        </div>
      </div>

      <div aria-hidden="true">
        <GoalFieldSkeleton />
      </div>

      <div aria-hidden="true">
        <div className="mb-3 flex h-lh items-center">
          <div className="h-5 w-24 rounded bg-gray-200" />
        </div>
        <div className="space-y-3">
          {Array.from({ length: 4 }, (_, index) => (
            <div key={index} className="flex gap-2">
              <div className="h-10 min-w-0 flex-1 rounded-xl bg-gray-200" />
              <div className="h-9 w-28 shrink-0 rounded-xl bg-gray-200" />
            </div>
          ))}
        </div>
        <div className="mt-3">
          <div className="h-10 rounded-xl bg-gray-200" />
        </div>
      </div>
    </div>
  );
}

function GoalFieldSkeleton(props: { helper?: boolean }) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex h-5 items-center">
        <div className="h-4 w-24 rounded bg-gray-200" />
      </div>
      <div className="h-10 rounded-xl bg-gray-200" />
      {props.helper && (
        <div className="flex h-5 items-center">
          <div className="h-4 w-40 rounded bg-gray-200" />
        </div>
      )}
    </div>
  );
}
