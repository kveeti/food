use topcoat::{
    Result,
    view::{Attributes, StaticClass, class, component, view},
};

const INPUT: StaticClass = class!(
    "w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 outline-2 \
     outline-transparent outline-offset-2 placeholder:text-gray-400 focus:border-gray-500 \
     focus-visible:outline-outline dark:border-gray-700 dark:bg-gray-900 \
     dark:focus:border-gray-500",
);

#[component]
pub async fn input(#[default] label: Option<&str>, #[default] mut attrs: Attributes) -> Result {
    let control = view! { <input class=(class!(INPUT, attrs.remove("class"))) (attrs)> }?;

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
