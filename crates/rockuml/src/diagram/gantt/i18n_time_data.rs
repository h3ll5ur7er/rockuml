//! The names PlantUML gives days and months in each language it knows (`I18nTimeData`), English when it
//! knows none.

/// Names in a few languages, by language code, and the English ones for the others.
pub(super) struct Names<const N: usize> {
    by_language: &'static [(&'static str, [&'static str; N])],
    fallback: [&'static str; N],
}

impl<const N: usize> Names<N> {
    pub(super) fn get(&self, index: usize, language: &str) -> &'static str {
        self.by_language
            .iter()
            .find(|(code, _)| *code == language)
            .map_or(self.fallback[index], |(_, names)| names[index])
    }
}

pub(super) const DAY_OF_WEEK_SHORT: Names<7> = Names {
    by_language: &[
        ("de", ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]),
        ("es", ["lu", "ma", "mi", "ju", "vi", "sá", "do"]),
        ("fr", ["lu", "ma", "me", "je", "ve", "sa", "di"]),
        ("ja", ["月", "火", "水", "木", "金", "土", "日"]),
        ("ko", ["월", "화", "수", "목", "금", "토", "일"]),
        ("ru", ["пн", "вт", "ср", "чт", "пт", "сб", "вс"]),
        (
            "zh",
            ["周一", "周二", "周三", "周四", "周五", "周六", "周日"],
        ),
    ],
    fallback: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
};

pub(super) const MONTH_SHORT: Names<12> = Names {
    by_language: &[
        (
            "de",
            [
                "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
            ],
        ),
        (
            "es",
            [
                "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
            ],
        ),
        (
            "fr",
            [
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
                "nov.", "déc.",
            ],
        ),
        (
            "ja",
            [
                "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                "12月",
            ],
        ),
        (
            "ko",
            [
                "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월",
                "12월",
            ],
        ),
        (
            "ru",
            [
                "янв.",
                "февр.",
                "март",
                "апр.",
                "май",
                "июнь",
                "июль",
                "авг.",
                "сент.",
                "окт.",
                "нояб.",
                "дек.",
            ],
        ),
        (
            "zh",
            [
                "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                "12月",
            ],
        ),
    ],
    fallback: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
};

pub(super) const MONTH_LONG: Names<12> = Names {
    by_language: &[
        (
            "de",
            [
                "Januar",
                "Februar",
                "März",
                "April",
                "Mai",
                "Juni",
                "Juli",
                "August",
                "September",
                "Oktober",
                "November",
                "Dezember",
            ],
        ),
        (
            "es",
            [
                "enero",
                "febrero",
                "marzo",
                "abril",
                "mayo",
                "junio",
                "julio",
                "agosto",
                "septiembre",
                "octubre",
                "noviembre",
                "diciembre",
            ],
        ),
        (
            "fr",
            [
                "janvier",
                "février",
                "mars",
                "avril",
                "mai",
                "juin",
                "juillet",
                "août",
                "septembre",
                "octobre",
                "novembre",
                "décembre",
            ],
        ),
        (
            "ja",
            [
                "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                "12月",
            ],
        ),
        (
            "ko",
            [
                "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월",
                "12월",
            ],
        ),
        (
            "ru",
            [
                "январь",
                "февраль",
                "март",
                "апрель",
                "май",
                "июнь",
                "июль",
                "август",
                "сентябрь",
                "октябрь",
                "ноябрь",
                "декабрь",
            ],
        ),
        (
            "zh",
            [
                "一月",
                "二月",
                "三月",
                "四月",
                "五月",
                "六月",
                "七月",
                "八月",
                "九月",
                "十月",
                "十一月",
                "十二月",
            ],
        ),
    ],
    fallback: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
};
