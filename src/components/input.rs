use topcoat::{
    Result,
    view::{Attributes, StaticClass, class, component, view},
};

const INPUT: StaticClass = class!(
    "w-full rounded-lg border border-gray-300 bg-white outline-2 \
     outline-transparent outline-offset-2 placeholder:text-gray-400 focus:border-gray-500 \
     focus-visible:outline-outline dark:border-gray-700 dark:bg-gray-900 \
     dark:focus:border-gray-500",
);

#[component]
pub async fn input(
    #[default] label: Option<&str>,
    #[default] suffix: Option<&str>,
    #[default] compact: bool,
    #[default] mut attrs: Attributes,
) -> Result {
    let control = view! {
        <span class="relative block">
            <input
                class=(class!(
                    INPUT,
                    "px-2.5 py-1.5" if compact,
                    "px-3 py-2.5" if !compact,
                    "pr-7" if compact && suffix.is_some(),
                    "pr-10" if !compact && suffix.is_some(),
                    attrs.remove("class"),
                ))
                (attrs)
            >
            if let Some(suffix) = suffix {
                <span
                    aria-hidden="true"
                    class=(class!(
                        "pointer-events-none absolute inset-y-0 flex items-center text-sm text-gray-500 dark:text-gray-400",
                        "right-2" if compact,
                        "right-3" if !compact,
                    ))
                >
                    (suffix)
                </span>
            }
        </span>
    }?;

    view! {
        if let Some(label) = label {
            <label class="block">
                <span class="mb-2 block text-sm font-medium">(label)</span>
                (control)
            </label>
        } else {
            (control)
        }
    }
}
