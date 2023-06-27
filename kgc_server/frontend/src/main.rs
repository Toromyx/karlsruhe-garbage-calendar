use wasm_bindgen::JsCast;
use web_sys::{window, HtmlInputElement, Url, UrlSearchParams};
use yew::prelude::*;

#[function_component]
pub fn App() -> Html {
    let street_handle = use_state_eq(|| String::from(""));
    let street = (*street_handle).clone();
    let street_number_handle = use_state_eq(|| String::from(""));
    let street_number = (*street_number_handle).clone();

    let exclude_residual_handle = use_state_eq(|| false);
    let exclude_residual = *exclude_residual_handle;
    let exclude_organic_handle = use_state_eq(|| false);
    let exclude_organic = *exclude_organic_handle;
    let exclude_recyclable_handle = use_state_eq(|| false);
    let exclude_recyclable = *exclude_recyclable_handle;
    let exclude_paper_handle = use_state_eq(|| false);
    let exclude_paper = *exclude_paper_handle;
    let exclude_bulky_handle = use_state_eq(|| false);
    let exclude_bulky = *exclude_bulky_handle;

    let calendar_url_search_params = |street: &str, street_number: &str| -> UrlSearchParams {
        let url_search_params = UrlSearchParams::new().unwrap();
        url_search_params.set("street", street);
        url_search_params.set("street_number", street_number);
        url_search_params
    };
    let calendar_url = |path: &str, url_search_params: UrlSearchParams| -> String {
        let url = Url::new_with_base(
            path,
            &String::from(window().unwrap().location().to_string()),
        )
        .unwrap();
        url.set_search(&String::from(url_search_params.to_string()));
        String::from(url.to_string())
    };
    let specific_calendar_url = |street: &str, street_number: &str, r#type: &str| -> String {
        let url_search_params = calendar_url_search_params(street, street_number);
        calendar_url(&format!("/calendar/{}", r#type), url_search_params)
    };

    let main_url_handle = use_memo(
        |(
            street,
            street_number,
            exclude_residual,
            exclude_organic,
            exclude_recyclable,
            exclude_paper,
            exclude_bulky,
        )| {
            let url_search_params = calendar_url_search_params(street, street_number);
            if *exclude_residual {
                url_search_params.set("exclude_residual", "true");
            }
            if *exclude_organic {
                url_search_params.set("exclude_organic", "true");
            }
            if *exclude_recyclable {
                url_search_params.set("exclude_recyclable", "true");
            }
            if *exclude_paper {
                url_search_params.set("exclude_paper", "true");
            }
            if *exclude_bulky {
                url_search_params.set("exclude_bulky", "true");
            }
            calendar_url("/calendar", url_search_params)
        },
        (
            street.clone(),
            street_number.clone(),
            exclude_residual,
            exclude_organic,
            exclude_recyclable,
            exclude_paper,
            exclude_bulky,
        ),
    );
    let main_url = (*main_url_handle).clone();

    let residual_url_handle = use_memo(
        |(street, street_number)| specific_calendar_url(street, street_number, "residual"),
        (street.clone(), street_number.clone()),
    );
    let residual_url = (*residual_url_handle).clone();
    let organic_url_handle = use_memo(
        |(street, street_number)| specific_calendar_url(street, street_number, "organic"),
        (street.clone(), street_number.clone()),
    );
    let organic_url = (*organic_url_handle).clone();
    let recyclable_url_handle = use_memo(
        |(street, street_number)| specific_calendar_url(street, street_number, "recyclable"),
        (street.clone(), street_number.clone()),
    );
    let recyclable_url = (*recyclable_url_handle).clone();
    let paper_url_handle = use_memo(
        |(street, street_number)| specific_calendar_url(street, street_number, "paper"),
        (street.clone(), street_number.clone()),
    );
    let paper_url = (*paper_url_handle).clone();
    let bulky_url_handle = use_memo(
        |(street, street_number)| specific_calendar_url(street, street_number, "bulky"),
        (street.clone(), street_number.clone()),
    );
    let bulky_url = (*bulky_url_handle).clone();

    let on_input_street = Callback::from(move |e: InputEvent| {
        street_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .value(),
        )
    });
    let on_input_street_number = Callback::from(move |e: InputEvent| {
        street_number_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .value(),
        )
    });
    let on_input_exclude_residual = Callback::from(move |e: InputEvent| {
        exclude_residual_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .checked(),
        )
    });
    let on_input_exclude_organic = Callback::from(move |e: InputEvent| {
        exclude_organic_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .checked(),
        )
    });
    let on_input_exclude_recyclable = Callback::from(move |e: InputEvent| {
        exclude_recyclable_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .checked(),
        )
    });
    let on_input_exclude_paper = Callback::from(move |e: InputEvent| {
        exclude_paper_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .checked(),
        )
    });
    let on_input_exclude_bulky = Callback::from(move |e: InputEvent| {
        exclude_bulky_handle.set(
            e.target()
                .unwrap()
                .unchecked_into::<HtmlInputElement>()
                .checked(),
        )
    });

    html! {
        <main>
            <div>
                <label>{"Street"}<br/><input
                    oninput={on_input_street}
                    name="street"
                    placeholder="Schloßplatz"
                    value={street.clone()}
                /></label>
            </div>
            <div>
                <label>{"Street number"}<br/><input
                    oninput={on_input_street_number}
                    name="street_number"
                    placeholder="1"
                    value={street_number.clone()}
                /></label>
            </div>
            <fieldset>
                <legend>{"Excluded waste types"}</legend>
                <div>
                    <label>
                        <input
                            oninput={on_input_exclude_residual}
                            name="exclude_residual"
                            type="checkbox"
                            checked={exclude_residual}
                        />{"Residual"}</label
                    >
                </div>
                <div>
                    <label>
                        <input
                            oninput={on_input_exclude_organic}
                            name="exclude_organic"
                            type="checkbox"
                            checked={exclude_organic}
                        />{"Organic"}</label
                    >
                </div>
                <div>
                    <label>
                        <input
                            oninput={on_input_exclude_recyclable}
                            name="exclude_recyclable"
                            type="checkbox"
                            checked={exclude_recyclable}
                        />{"Recyclable"}</label
                    >
                </div>
                <div>
                    <label>
                        <input
                            oninput={on_input_exclude_paper}
                            name="exclude_paper"
                            type="checkbox"
                            checked={exclude_paper}
                        />{"Paper"}</label
                    >
                </div>
                <div>
                    <label>
                        <input
                            oninput={on_input_exclude_bulky}
                            name="exclude_bulky"
                            type="checkbox"
                            checked={exclude_bulky}
                        />{"Bulky"}</label
                    >
                </div>
            </fieldset>
            <output>
                <div>
                    <label>{"Main URL"}<br/><input
                        readonly=true
                        value={main_url.clone()}
                        style="width:100%"
                    /></label>
                </div>
                <div>
                    <label>{"Residual URL"}<br/><input
                        readonly=true
                        value={residual_url.clone()}
                        style="width:100%"
                    /></label>
                </div>
                <div>
                    <label>{"Organic URL"}<br/><input
                        readonly=true
                        value={organic_url.clone()}
                        style="width:100%"
                    /></label>
                </div>
                <div>
                    <label>{"Recyclable URL"}<br/><input
                        readonly=true
                        value={recyclable_url.clone()}
                        style="width:100%"
                    /></label>
                </div>
                <div>
                    <label>{"Paper URL"}<br/><input
                        readonly=true
                        value={paper_url.clone()}
                        style="width:100%"
                    /></label>
                </div>
                <div>
                    <label>{"Bulky URL"}<br/><input
                        readonly=true
                        value={bulky_url.clone()}
                        style="width:100%"
                    /></label>
                </div>
            </output>
        </main>
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    yew::Renderer::<App>::new().render();
}
