use harper_core::Document;
use harper_core::linting::*;
use harper_core::spell::FstDictionary;
use regex::Regex;
use std::io;
use std::io::Read;
use std::sync::Arc;
use std::thread;
#[derive(Copy, Clone)]
struct Grade {
    val: Option<bool>,
}
impl Grade {
    fn get(&self) -> bool {
        match self.val {
            Some(v) => v,
            _ => false,
        }
    }
    fn perc(&self) -> f32 {
        if self.get() { 1.0 } else { 0.0 }
    }
    fn fail(&mut self) {
        self.val = match self.val {
            Some(_) => self.val,
            None => Some(false),
        };
    }
    fn pass(&mut self) {
        self.val = match self.val {
            Some(_) => self.val,
            None => Some(true),
        };
    }
    fn empty() -> Grade {
        Grade { val: None }
    }
    fn new(v: bool) -> Grade {
        Grade { val: Some(v) }
    }
}
enum LintCategory {
    Punctuation,
    Spelling,
    Capitalization,
}

fn bucket_lints(text: &str) -> Vec<LintCategory> {
    let doc: Document = Document::new_plain_english_curated(text);
    let mut linter: LintGroup = LintGroup::default();
    let dict = FstDictionary::curated();
    let spellcheck: SpellCheck<Arc<FstDictionary>> =
        SpellCheck::new(dict.clone(), harper_core::Dialect::American);
    linter.add("Spelling", spellcheck);
    linter.add("AnA", AnA::default());
    linter.add(
        "CapitalizePersonalPronouns",
        CapitalizePersonalPronouns::default(),
    );
    linter.add("CommaFixes", CommaFixes::default());
    linter.add("CompoundNouns", CompoundNouns::default());
    linter.add("CorrectNumberSuffix", CorrectNumberSuffix::default());
    linter.add("CurrencyPlacement", CurrencyPlacement::default());
    linter.add("DiscourseMarkers", DiscourseMarkers::default());
    linter.add("EllipsisLength", EllipsisLength::default());
    linter.add("HopHope", HopHope::default());
    linter.add("ItsContraction", ItsContraction::default());
    linter.add("LetsConfusion", LetsConfusion::default());
    linter.add("NounVerbConfusion", NounVerbConfusion::default());
    linter.add(
        "NumberSuffixCapitalization",
        NumberSuffixCapitalization::default(),
    );
    linter.add(
        "PhrasalVerbAsCompoundNoun",
        PhrasalVerbAsCompoundNoun::default(),
    );
    linter.add("PronounContraction", PronounContraction::default());
    linter.add("UnclosedQuotes", UnclosedQuotes::default());
    linter.add(
        "InflectedVerbAfterTo",
        InflectedVerbAfterTo::new(dict.clone()),
    );
    linter.add(
        "SentenceCapitalization",
        SentenceCapitalization::new(dict.clone()),
    );

    linter.set_all_rules_to(Some(true));
    let lints = linter.lint(&doc);
    let mut buckets: Vec<LintCategory> = Vec::new();
    for error in lints {
        let error = error.lint_kind;
        buckets.push(match error {
            LintKind::BoundaryError => LintCategory::Spelling,
            LintKind::Capitalization => LintCategory::Capitalization,
            LintKind::Eggcorn => LintCategory::Spelling,
            LintKind::Malapropism => LintCategory::Spelling,
            LintKind::Punctuation => LintCategory::Punctuation,
            LintKind::Spelling => LintCategory::Spelling,
            LintKind::Typo => LintCategory::Spelling,
            _ => continue,
        })
    }
    buckets
}
struct Rubric {
    link: Grade,
    caps: Grade,
    punc: Grade,
    spel: Grade,
    ques: Grade,
}
impl Rubric {
    fn get(&mut self) -> f32 {
        (self.link.perc()
            + self.caps.perc()
            + self.punc.perc()
            + self.spel.perc()
            + self.ques.perc())
            / 5.0
    }
    fn new() -> Rubric {
        Rubric {
            link: Grade::empty(),
            caps: Grade::empty(),
            punc: Grade::empty(),
            spel: Grade::empty(),
            ques: Grade::empty(),
        }
    }
    fn from_string(contents: String) -> Rubric {
        fn punc_spell_caps(contents: &String) -> (bool, bool, bool) {
            let lints = bucket_lints(contents);
            let mut punc = Grade::empty();
            let mut spel = Grade::empty();
            let mut caps = Grade::empty();
            for lint in lints {
                match lint {
                    LintCategory::Punctuation => punc.fail(),
                    LintCategory::Spelling => spel.fail(),
                    LintCategory::Capitalization => caps.fail(),
                }
            }
            punc.pass();
            spel.pass();
            caps.pass();
            return (punc.get(), spel.get(), caps.get());
        }
        fn contains_link(contents: &String) -> bool {
            let regex = Regex::new(r"((youtube.com)|(youtu.be)|(tiktok.com))\/").unwrap();
            regex.is_match(contents)
        }

        let mut out = Rubric::new();
        let contents_clone = contents.clone();
        let handle = thread::spawn(move || punc_spell_caps(&contents_clone));
        out.link = Grade::new(contains_link(&contents));
        println!("{}", contents);
        println!("Complete sentences and all questions answered?");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("failed to read input");
        input = input.trim().to_string();
        input = input.to_lowercase().to_string();
        if input.chars().nth(0).unwrap_or('y') == 'y' {
            out.ques.pass();
        } else {
            out.ques.fail();
        }
        let psc = handle.join().expect("failed to lint");
        out.punc = Grade::new(psc.0);
        out.spel = Grade::new(psc.1);
        out.caps = Grade::new(psc.2);
        return out;
    }
    fn output(&mut self) -> String {
        let score = self.get();
        let mut out = String::new();
        out += &format!(
            "{}%(20%): Contains a link to a youtube video\n",
            self.link.perc() * 20.0
        )
        .to_string();
        out += &format!("{}%(20%): No spelling mistakes\n", self.spel.perc() * 20.0);
        out += &format!(
            "{}%(20%): No punctuation mistakes\n",
            self.punc.perc() * 20.0
        );
        out += &format!(
            "{}%(20%): No capitalization mistakes\n",
            self.caps.perc() * 20.0
        )
        .to_string();
        out += &format!(
            "{}%(20%): Answered all the questions in complete sentences\n",
            self.ques.perc() * 20.0
        )
        .to_string();
        out += &"#== === === === =#= === === === ==#\n".to_string();
        out += &format!("{}%(100%): Final score\n", (score * 100.0).round()).to_string();
        out
    }
}
fn main() {
    let mut f = std::fs::File::open(std::path::Path::new("input.txt")).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    println!("{}", Rubric::from_string(contents).output());
}
