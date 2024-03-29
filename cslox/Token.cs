public enum TokenType
{
    PAREN_O,
    PAREN_C,
    BRACKET_O,
    BRACKET_C,
    COMMA,
    DOT,
    MINUS,
    PLUS,
    SEMICOLON,
    FWDSLASH,
    ASTERISK,
    NOT,
    NOT_EQ,
    EQ,
    DBL_EQ,
    GT,
    GTE,
    LT,
    LTE,
    IDENTIFIER,
    STRING,
    NUMBER,
    AND,
    CLASS,
    ELSE,
    FALSE,
    FUN,
    FOR,
    IF,
    NIL,
    OR,
    PRINT,
    RETURN,
    SUPER,
    THIS,
    TRUE,
    VAR,
    WHILE,
    EOF,
}

enum TokenCat
{
    Structural,
    Assign,
    Op,
    Comp,
    Keyword,
    Literal,
}

public class Token(TokenType type, string token, object? literal, int line)
{
    public TokenType type = type;
    public string token = token;
    public object? literal = literal;
    public int line = line;

    public override string ToString() {
        return $"{type} {token} {literal}";
    }
}