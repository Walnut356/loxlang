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

public class Token {
    public TokenType type;
    public string token;
    public object? literal;
    public int line;

    public Token(TokenType type, string token, object? literal, int line) {
        this.type = type;
        this.token = token;
        this.literal = literal;
        this.line = line;
    }

    public override string ToString() {
        return $"{type} {token} {literal}";
    }
}