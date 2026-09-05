use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

const SELECT: StaticClass = class!(
    "h-11 w-full appearance-none rounded-lg border border-gray-300 bg-white px-3 pr-10 \
     text-sm outline-2 outline-transparent outline-offset-2 focus:border-gray-500 \
     focus-visible:outline-outline disabled:cursor-not-allowed disabled:opacity-60 \
     dark:border-gray-700 dark:bg-gray-900 dark:focus:border-gray-500",
);

#[component]
pub async fn select(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <span class="relative block">
            <select class=(class!(SELECT, attrs.remove("class"))) (attrs)>
                (child)
            </select>
            <span
                aria-hidden="true"
                class="pointer-events-none absolute inset-y-0 right-3 flex items-center text-gray-500 dark:text-gray-400"
            >
                <svg viewBox="0 0 16 16" class="size-4" fill="none">
                    <path
                        d="m4 6 4 4 4-4"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    ></path>
                </svg>
            </span>
        </span>
    }
}
