use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

#[derive(Clone, Copy, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Accent,
    Nav,
}

impl ButtonVariant {
    fn classes(self) -> StaticClass {
        match self {
            Self::Primary => class!(
                "h-10 rounded-lg bg-gray-900 px-5 text-sm font-medium text-white \
                 outline-2 outline-transparent outline-offset-2 hover:bg-gray-700 \
                 focus-visible:outline-outline dark:bg-gray-100 dark:text-gray-950 \
                 dark:hover:bg-white",
            ),
            Self::Accent => class!(
                "css-squircle-button h-12 bg-water px-5 text-sm font-medium text-blue-950 \
                 outline-2 outline-transparent outline-offset-2 hover:bg-water-hover \
                 focus-visible:outline-outline",
            ),
            Self::Nav => class!(
                "inline-flex h-full items-center px-3 text-sm text-inherit outline-2 \
                 outline-transparent outline-offset-[-2px] hover:bg-gray-200/80 \
                 focus-visible:outline-outline dark:hover:bg-gray-800/80",
            ),
        }
    }
}

#[component]
pub async fn button(
    #[default] variant: ButtonVariant,
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <button class=(class!(variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </button>
    }
}
