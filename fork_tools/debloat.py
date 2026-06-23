#!/usr/bin/env python3
from pathlib import Path
import tomllib as tl
import os

type NameList = list[str]
type ItemList = list[dict]
cwd = os.getcwd()

DEL_LANG: list[str] = ["vim"]


def read_unused_languages() -> list[str]:
    fn = Path(cwd) / "unused_languages.txt"
    return sorted(fn.read_text().strip().splitlines())


def read_languages_toml() -> dict:
    return tl.loads(Path("../languages.toml").read_text())


def debloat_grammars(): ...


if __name__ == "__main__":
    debloat_grammars()
    print(cwd)
    t = read_languages_toml()
    # print(t.keys())

    exclude = read_unused_languages()

    use_grammars: ItemList = t["use-grammars"]
    language_server: ItemList = t["language-server"]
    language: ItemList = t["language"]
    grammar: ItemList = t["grammar"]

    all_grammars: NameList = [g["name"] for g in grammar]
    keep_grammars: NameList = sorted(list(set(all_grammars) - set(exclude)))

    # print(f"{use_grammars = }")

    # print(f"{language_server = }")
    # print(f"{language = }")

    # print(f"{grammar = }")

    # we want to check which languages do not have the field grammar and
    # if there is at least one language without the field and without its
    # name in the list of grammars. That would mean that there is no associated grammar
    # So far only: `llvm-mir-yaml`
    for lang in language:
        if lang["name"] in exclude:
            continue

        if not "grammar" in lang and not lang["name"] in all_grammars:
            print(lang)

    # we want to check which languages do not have the field grammar and
    # if those languages can be found by name in the list of grammars.
    # We could add the field grammar and make the data more consistent
    # across languages
    #
    print()
    for lang in language:
        if lang["name"] in exclude:
            continue

        if not "grammar" in lang and lang["name"] in all_grammars:
            print(lang)

    # print(exclude)
    # print()
    # print(keep_grammars)

    # for ls in language_server:
    # print(ls)
    # print(language_server)
    # print(language)
    # filtered_language: ItemList = []
    # for lang in language:
    #     if "grammar" not in lang:
    #         filtered_language.append(lang)
    #         continue
    #     # print(lang)
    # print(filtered_language)
