// Enum values UHT leaves unparsed.
//
// UHT records an enumerator's value only when it is a plain integer literal;
// for anything else (`(1 << 0)`, `X | Y`, `BLEND_Translucent`) it stores -1,
// and so does every implicit value after it. Its own generated code only needs
// the names, but the Rust bindings need the numbers, so they are recomputed
// here from the enum's declaration in its header.

#nullable disable

using EpicGames.UHT.Types;

namespace Rusteal;

internal static class RustealEnumValues
{
    /// <summary>
    /// Every enumerator of the enum, by name, with the value the compiler gives
    /// it; null when the declaration cannot be read. An enumerator whose
    /// initializer cannot be evaluated (a macro, another enum's value) is left
    /// out, along with the implicit values that follow it.
    /// </summary>
    public static Dictionary<string, long> Evaluate(UhtEnum enumObj)
    {
        try
        {
            string body = FindBody(enumObj.HeaderFile.Data.ToString(), enumObj);
            return body == null ? null : EvaluateBody(body, enumObj.SourceName);
        }
        catch (FormatException)
        {
            return null;
        }
    }

    // The text between the braces of the enum's declaration, comments removed.
    private static string FindBody(string source, UhtEnum enumObj)
    {
        int lineStart = 0;
        for (int line = 1; line < enumObj.LineNumber && lineStart >= 0; line++)
        {
            lineStart = source.IndexOf('\n', lineStart);
            if (lineStart >= 0)
            {
                lineStart++;
            }
        }
        if (lineStart < 0)
        {
            return null;
        }

        string text = StripComments(source[lineStart..]);
        int nameAt = IndexOfWord(text, enumObj.SourceName, 0);
        if (nameAt < 0)
        {
            return null;
        }
        int open = text.IndexOf('{', nameAt);
        // `namespace EFoo { enum Type { ... } }`: the enum's own braces come next.
        if (open >= 0 && IndexOfWord(text[..open], "namespace", 0) >= 0)
        {
            int enumAt = IndexOfWord(text, "enum", open);
            open = enumAt < 0 ? -1 : text.IndexOf('{', enumAt);
        }
        if (open < 0)
        {
            return null;
        }
        int close = text.IndexOf('}', open);
        return close < 0 ? null : text[(open + 1)..close];
    }

    private static Dictionary<string, long> EvaluateBody(string body, string enumName)
    {
        if (body.Contains('#'))
        {
            return null; // preprocessor lines: which enumerators exist depends on the build
        }

        Dictionary<string, long> values = [];
        long next = 0;
        bool known = true;
        foreach (string rawEntry in SplitTopLevel(RemoveUMeta(body)))
        {
            string entry = rawEntry.Trim();
            if (entry.Length == 0)
            {
                continue;
            }
            // The name is the leading identifier: an attribute such as
            // UE_DEPRECATED(...) may sit between it and the `=`.
            int nameEnd = 0;
            while (nameEnd < entry.Length && IsIdentChar(entry[nameEnd]))
            {
                nameEnd++;
            }
            string name = entry[..nameEnd];
            int eq = entry.IndexOf('=', nameEnd);
            if (eq >= 0)
            {
                known = new ExpressionParser(entry[(eq + 1)..], values, enumName).TryParse(out next);
            }
            if (known)
            {
                values[name] = next;
                next++;
            }
        }
        return values;
    }

    private static string StripComments(string text)
    {
        var result = new System.Text.StringBuilder(text.Length);
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] == '/' && i + 1 < text.Length && text[i + 1] == '/')
            {
                int end = text.IndexOf('\n', i);
                i = end < 0 ? text.Length : end - 1;
            }
            else if (text[i] == '/' && i + 1 < text.Length && text[i + 1] == '*')
            {
                int end = text.IndexOf("*/", i + 2, StringComparison.Ordinal);
                i = end < 0 ? text.Length : end + 1;
                result.Append(' ');
            }
            else
            {
                result.Append(text[i]);
            }
        }
        return result.ToString();
    }

    // `UMETA(...)` after an enumerator, balanced parentheses included.
    private static string RemoveUMeta(string body)
    {
        int at;
        while ((at = IndexOfWord(body, "UMETA", 0)) >= 0)
        {
            int open = body.IndexOf('(', at);
            int depth = 0;
            int end = open;
            for (; end >= 0 && end < body.Length; end++)
            {
                if (body[end] == '(')
                {
                    depth++;
                }
                else if (body[end] == ')' && --depth == 0)
                {
                    break;
                }
            }
            if (open < 0 || end >= body.Length)
            {
                throw new FormatException("unbalanced UMETA");
            }
            body = body[..at] + body[(end + 1)..];
        }
        return body;
    }

    private static IEnumerable<string> SplitTopLevel(string body)
    {
        int depth = 0;
        int start = 0;
        for (int i = 0; i < body.Length; i++)
        {
            switch (body[i])
            {
                case '(':
                    depth++;
                    break;
                case ')':
                    depth--;
                    break;
                case ',' when depth == 0:
                    yield return body[start..i];
                    start = i + 1;
                    break;
            }
        }
        yield return body[start..];
    }

    private static int IndexOfWord(string text, string word, int from)
    {
        for (int at = text.IndexOf(word, from, StringComparison.Ordinal); at >= 0;
             at = text.IndexOf(word, at + 1, StringComparison.Ordinal))
        {
            bool startOk = at == 0 || !IsIdentChar(text[at - 1]);
            int end = at + word.Length;
            bool endOk = end == text.Length || !IsIdentChar(text[end]);
            if (startOk && endOk)
            {
                return at;
            }
        }
        return -1;
    }

    private static bool IsIdentChar(char c) => char.IsLetterOrDigit(c) || c == '_';

    // A C++ integer constant expression over literals and earlier enumerators:
    // | ^ & << >> + - * / % and the unary - ~ !, with C++ precedence.
    private sealed class ExpressionParser(string text, Dictionary<string, long> names, string enumName)
    {
        private int _pos;

        public bool TryParse(out long value)
        {
            try
            {
                value = ParseBinary(0);
                SkipSpace();
                return _pos == text.Length;
            }
            catch (FormatException)
            {
                value = 0;
                return false;
            }
        }

        private static readonly string[][] Levels =
        [
            ["|"],
            ["^"],
            ["&"],
            ["<<", ">>"],
            ["+", "-"],
            ["*", "/", "%"],
        ];

        private long ParseBinary(int level)
        {
            if (level == Levels.Length)
            {
                return ParseUnary();
            }
            long left = ParseBinary(level + 1);
            while (true)
            {
                SkipSpace();
                string op = Levels[level].FirstOrDefault(IsOperatorAt);
                if (op == null)
                {
                    return left;
                }
                _pos += op.Length;
                long right = ParseBinary(level + 1);
                left = op switch
                {
                    "|" => left | right,
                    "^" => left ^ right,
                    "&" => left & right,
                    "<<" => left << (int)right,
                    ">>" => left >> (int)right,
                    "+" => left + right,
                    "-" => left - right,
                    "*" => left * right,
                    "/" when right != 0 => left / right,
                    "%" when right != 0 => left % right,
                    _ => throw new FormatException("division by zero"),
                };
            }
        }

        // `|` is not `||`, `&` is not `&&`, `<<` is not `<<=`.
        private bool IsOperatorAt(string op)
        {
            if (string.CompareOrdinal(text, _pos, op, 0, op.Length) != 0)
            {
                return false;
            }
            int after = _pos + op.Length;
            return after == text.Length || (text[after] != op[^1] && text[after] != '=');
        }

        private long ParseUnary()
        {
            SkipSpace();
            if (_pos >= text.Length)
            {
                throw new FormatException("unexpected end");
            }
            switch (text[_pos])
            {
                case '-':
                    _pos++;
                    return -ParseUnary();
                case '+':
                    _pos++;
                    return ParseUnary();
                case '~':
                    _pos++;
                    return ~ParseUnary();
                case '!':
                    _pos++;
                    return ParseUnary() == 0 ? 1 : 0;
                case '(':
                    _pos++;
                    long inner = ParseBinary(0);
                    SkipSpace();
                    if (_pos >= text.Length || text[_pos] != ')')
                    {
                        throw new FormatException("missing )");
                    }
                    _pos++;
                    return inner;
            }
            return char.IsDigit(text[_pos]) ? ParseNumber() : ParseName();
        }

        private long ParseNumber()
        {
            int start = _pos;
            while (_pos < text.Length && (char.IsLetterOrDigit(text[_pos]) || text[_pos] == '\''))
            {
                _pos++;
            }
            string literal = text[start.._pos].Replace("'", "").TrimEnd('u', 'U', 'l', 'L');
            if (literal.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
            {
                return Convert.ToInt64(literal[2..], 16);
            }
            if (literal.StartsWith("0b", StringComparison.OrdinalIgnoreCase))
            {
                return Convert.ToInt64(literal[2..], 2);
            }
            if (literal.Length > 1 && literal[0] == '0')
            {
                return Convert.ToInt64(literal[1..], 8);
            }
            return long.Parse(literal, System.Globalization.CultureInfo.InvariantCulture);
        }

        // An earlier enumerator of this enum, bare or qualified by it
        // (`EFoo::Bar`, `EFoo::Type::Bar`).
        private long ParseName()
        {
            int start = _pos;
            while (_pos < text.Length && (IsIdentChar(text[_pos]) || text[_pos] == ':'))
            {
                _pos++;
            }
            string name = text[start.._pos];
            int colons = name.LastIndexOf("::", StringComparison.Ordinal);
            if (colons >= 0 && name.Split("::")[0] != enumName)
            {
                throw new FormatException($"another type's constant {name}");
            }
            string last = colons < 0 ? name : name[(colons + 2)..];
            return names.TryGetValue(last, out long value)
                ? value
                : throw new FormatException($"unknown name {name}");
        }

        private void SkipSpace()
        {
            while (_pos < text.Length && char.IsWhiteSpace(text[_pos]))
            {
                _pos++;
            }
        }
    }
}
