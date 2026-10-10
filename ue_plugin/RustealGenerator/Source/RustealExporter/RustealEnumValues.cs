#nullable disable

using EpicGames.UHT.Types;

namespace Rusteal;

internal static class RustealEnumValues
{
    public static Dictionary<string, long> Evaluate(UhtEnum enumObj)
    {
        try
        {
            string body = FindBody(enumObj.HeaderFile.Data.ToString(), enumObj);
            ThirdPartyConstants.EngineDirectory ??= enumObj.Session.EngineDirectory;
            return body == null ? null : EvaluateBody(body, enumObj.SourceName);
        }
        catch (FormatException)
        {
            return null;
        }
    }

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
            return null;
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

            if (names.TryGetValue(last, out long value))
            {
                return value;
            }

            return colons < 0 && ThirdPartyConstants.TryResolve(name, out long external)
                ? external
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

    private static class ThirdPartyConstants
    {
        public static string EngineDirectory;

        private static List<string> _headers;
        private static readonly Dictionary<string, long?> Resolved = [];
        private static readonly HashSet<string> Resolving = [];

        public static bool TryResolve(string name, out long value)
        {
            value = 0;

            if (Resolved.TryGetValue(name, out long? cached))
            {
                value = cached ?? 0;
                return cached.HasValue;
            }

            if (EngineDirectory == null || !Resolving.Add(name))
            {
                return false;
            }

            long? found = null;

            try
            {
                foreach (string header in Headers())
                {
                    string text = File.ReadAllText(header);

                    if (!text.Contains(name, StringComparison.Ordinal))
                    {
                        continue;
                    }

                    var definition = System.Text.RegularExpressions.Regex.Match(
                        StripComments(text),
                        $@"(?:^|[\s,{{(])(?:#define\s+{name}\s+(?<expr>[^\n]+)|{name}\s*=\s*(?<expr>[^,\n}}]+))",
                        System.Text.RegularExpressions.RegexOptions.Multiline);

                    if (definition.Success
                        && new ExpressionParser(definition.Groups["expr"].Value.Trim(), [], "").TryParse(out long parsed))
                    {
                        found = parsed;
                        break;
                    }
                }
            }
            finally
            {
                Resolving.Remove(name);
            }

            Resolved[name] = found;
            value = found ?? 0;
            return found.HasValue;
        }

        private static List<string> Headers()
        {
            if (_headers != null)
            {
                return _headers;
            }

            string thirdParty = Path.Combine(EngineDirectory, "Source", "ThirdParty");

            _headers = Directory.Exists(thirdParty)
                ? Directory.EnumerateFiles(thirdParty, "*.h", SearchOption.AllDirectories)
                    .Where(path => path.Replace('\\', '/').Contains("/include/", StringComparison.Ordinal))
                    .ToList()
                : [];

            return _headers;
        }
    }
}
