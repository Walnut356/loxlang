using static Utils;
using static TokenType;
// using Token = Token.Token;

public class Scanner
{
    string code;
    List<Token> tokens = [];
    i32 line = 1;
    i32 start = 0;
    i32 i = 0;

    public Scanner(string code)
    {
        this.code = code;
    }

    public List<Token> ScanTokens()
    {
        while (i < code.Length)
        {
            start = i;
            char c = code[i];
            i++;
            switch (c)
            {
                case '(': AddToken(PAREN_O); break;
                case ')': AddToken(PAREN_C); break;
                case '{': AddToken(BRACKET_O); break;
                case '}': AddToken(BRACKET_C); break;
                case ',': AddToken(COMMA); break;
                case '.': AddToken(DOT); break;
                case '-': AddToken(MINUS); break;
                case '+': AddToken(PLUS); break;
                case ';': AddToken(SEMICOLON); break;
                case '*': AddToken(ASTERISK); break;
                case '!': AddToken(Match('=') ? NOT_EQ : NOT); break;
                case '=': AddToken(Match('=') ? DBL_EQ : EQ); break;
                case '<': AddToken(Match('=') ? LTE : LT); break;
                case '>': AddToken(Match('=') ? GTE : GT); break;
                case '/':
                    {

                        if (i < code.Length && code[i] == '/')
                        {
                            while (i < code.Length && code[i] != '\n') i++;
                            i++;
                        } else if (i < code.Length && code[i] == '*') {
                            i += 2;
                            var nesting = 1;
                            while (nesting > 0 && i < code.Length) {
                                if (code[i - 1] == '/' && code[i] == '*') {
                                    nesting += 1;
                                } else if (code [i - 1] == '*' && code[i] == '/') {
                                    nesting -= 1;
                                }
                                ++i;
                            }
                        }
                        else
                        {
                            AddToken(FWDSLASH);
                        }
                    }
                    break;
                case ' ':
                case '\t':
                case '\r': break;
                case '\n': line++; break;
                case '"': StrLiteral(); break;
                default:
                    if (IsDigit(c))
                    {
                        NumLiteral();
                    }
                    else if (IsAlpha(c))
                    {
                        AddIdentifier();
                    }
                    else
                    {
                        Lox.Error(line, $"Unexpected character: {c}");
                    }
                    break;
            }
        }

        tokens.Add(new Token(EOF, "", null, line));
        return tokens;
    }

    void AddToken(TokenType type, object? literal = null)
    {
        var text = code[start..i];
        tokens.Add(new Token(type, text, literal, line));
    }

    bool Match(char c)
    {
        if (i < code.Length && code[i] != c) return false;

        i++;
        return true;
    }

    void StrLiteral()
    {
        while (i < code.Length && code[i] != '"')
        {
            switch (code[i])
            {
                case '\n': line++; break;

            }

            i++;
        }

        if (i == code.Length)
        {
            Lox.Error(line, "Undetermined string");
            return;
        }
        i++;
        AddToken(STRING, code[(start + 1)..(i - 1)]);


    }

    void NumLiteral()
    {

        while (i < code.Length && IsDigit(code[i]))
        {
            i++;
        }

        if (i < code.Length && code[i] == '.' && i + 1 < code.Length && IsDigit(code[i + 1]))
        {
            i++;
            while (i < code.Length && IsDigit(code[i]))
            {
                i++;
            }
        }

        AddToken(NUMBER, f64.Parse(code[start..i]));
    }

    static readonly Dictionary<string, TokenType> keywords = new()
    {
        {"and", AND},
        {"class", CLASS},
        {"else", ELSE},
        {"false", FALSE},
        {"for", FOR},
        {"fun", FUN},
        {"if", IF},
        {"nil", NIL},
        {"or", OR},
        {"print", PRINT},
        {"return", RETURN},
        {"super", SUPER},
        {"this", THIS},
        {"true", TRUE},
        {"var", VAR},
        {"while", WHILE},
    };

    void AddIdentifier()
    {
        while (i < code.Length && IsAlpha(code[i]) || IsDigit(code[i]) || code[i] == '_')
        {
            i++;
        }

        if (keywords.TryGetValue(code[start..i], out var type))
        {
            AddToken(type);
        }
        else
        {
            AddToken(IDENTIFIER);
        }
    }
}