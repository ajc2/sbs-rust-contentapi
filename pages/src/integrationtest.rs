use common::*;
use common::render::layout::*;
use maud::html;

pub fn render(data: MainLayoutData) -> String 
{
    let mut errors : Vec<&'static str> = Vec::new();

    //Calculate errors before rendering page to make life easier
    if !data.links.http_root.is_ascii() { errors.push("Frontend isn't running locally"); }
    if data.user.is_some() { errors.push("You must be logged out"); }

    layout(&data, html!{
        style { r#"
            #testframe {
                width: 100%;
                height: 60vh;
            }  
        "# }
        (data.links.script("/forpage/integrationtest.js"))
        section {
            h1 { "TESTING PAGE" }
            p {
                "To run this testing page, the following must be met:"
                ul {
                    li { "Running frontend locally (localhost)" }
                    li { "Running backend locally" }
                    li { "Not logged in (try private window?)"}
                    li { "Email handler set to null" }
                    li { "Backdoor email endpoint active" }
                    li { "Pre-existing data available (at least structure, categories, etc. not a fresh database)" }
                }
            }
            @if errors.is_empty() {
                a href="#" #"teststart" { "Loading tests..." }
                p."aside" { "Tests can take a very long time" }
                iframe #"testframe" { }
            }
            @else {
                h3 { "This testing page isn't available because:" }
                ul."errors" {
                    @for error in errors {
                        li."error" { (error) }
                    }
                }
            }
        }
    }).into_string()
}