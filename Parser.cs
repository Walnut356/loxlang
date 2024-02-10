using System.Text.RegularExpressions;
using static TokenType;

class Parser {
    List<Token> tokens;
    int i = 0;

    public Parser(List<Token> tokens) {
        this.tokens = tokens;
    }

    public Expr? Parse() {
        try {
            return Expression();
        } catch (ParseError error) {
            Console.WriteLine(error);
            return null;
        }
    }

    Expr Expression() {
        return Equality();
    }

    Expr Equality() {
        Expr expression = Comparison();

        while (tokens[i].type is NOT_EQ or EQ) {
            Token op = tokens[i];
            ++i;
            Expr right = Comparison();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Comparison() {
        Expr expression = Term();

        while (tokens[i].type is GT or LT or GTE or LTE)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Term();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Term() {
        Expr expression = Factor();

        while (tokens[i].type is MINUS or PLUS)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Factor();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Factor() {
        Expr expression = Unary();

        while (tokens[i].type is ASTERISK or FWDSLASH)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Unary();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    static readonly TokenType[] NEG = [NOT, MINUS];

    Expr Unary() {
        if (NEG.Contains(tokens[i].type)) {
            Token op = tokens[i];
            i++;
            Expr right = Unary();
            return new Unary(op, right);
        }

        return Primary();
    }

    Expr Primary() {
        var token = tokens[i];
        i++;
        switch (token.type) {
            case FALSE:
                return new Literal(false);
            case TRUE:
                return new Literal(true);
            case NIL:
                return new Literal(null);
            case NUMBER:
                return new Literal(token.literal);
            case STRING:
                return new Literal(token.literal);
            case PAREN_O:
                Expr expr = Expression();
                Expect(PAREN_C, "Expected ')' after expression.");
                return new Grouping(expr);
            default:
                throw Error(token, "Expected expression");
        }
    }


    bool EOF() {
        return tokens[i].type == TokenType.EOF;
    }

    void Expect(TokenType t, string message) {
        if (tokens[i].type != t) { throw Error(tokens[i], message); }
        ++i;
    }

    class ParseError : Exception { }

    ParseError Error(Token token, string message) {
        Lox.Error(token, message);
        return new ParseError();
    }

    void Sync() {
        while (!EOF()) {
            if (tokens[i - 1].type == SEMICOLON) return;
            switch (tokens[i + 1].type) {
                case CLASS:
                case FUN:
                case VAR:
                case FOR:
                case IF:
                case WHILE:
                case PRINT:
                case RETURN:
                    return;
            }
            i++;
        }
    }
}