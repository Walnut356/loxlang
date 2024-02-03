using static TokenType;

class Parser {
    List<Token> tokens;
    int i = 0;



    public Parser(List<Token> tokens) {
        this.tokens = tokens;
    }

    Expr Expression() {
        return Equality();
    }

    static readonly TokenType[] N_EQ = [NOT_EQ, EQ];

    Expr Equality() {
        Expr left = Comparison();

        Expr? expression = null;
        while (tokens[i].type is NOT_EQ or EQ) {
            Token op = tokens[i];
            ++i;
            Expr right = Comparison();
            expression = new Binary(left, op, right);
        }

        return expression!;
    }

    Expr Comparison() {
        Expr left = Term();

        Expr? expression = null;
        while (tokens[i].type is GT or LT or GTE or LTE)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Term();
            expression = new Binary(left, op, right);
        }

        return expression!;
    }

    Expr Term() {
        Expr left = Factor();

        Expr? expression = null;
        while (tokens[i].type is MINUS or PLUS)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Factor();
            expression = new Binary(left, op, right);
        }

        return expression!;
    }

    Expr Factor() {
        Expr left = Unary();

        Expr? expression = null;
        while (tokens[i].type is ASTERISK or FWDSLASH)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Unary();
            expression = new Binary(left, op, right);
        }

        return expression!;
    }

    static readonly TokenType[] NEG = [NOT, MINUS];

    Expr Unary() {
        if (NEG.Contains(tokens[i].type)) {
            Token op = tokens[i];
            Expr right = Unary();
            return new Unary(op, right);
        }

        return Primary();
    }

    Expr Primary() {
        switch (tokens[i].type) {
            case FALSE:
                return new Literal(false);
            case TRUE:
                return new Literal(true);
            case NIL:
                return new Literal(null);
            case NUMBER:
                return new Literal(tokens[i].literal);
            case STRING:
                return new Literal(tokens[i].literal);
            case PAREN_O:
                Expr expr = Expression();
                if (tokens[i].type != PAREN_C) {
                    throw new Exception($"Expected close paren, got {tokens[i]}");
                }
                i++;
                return new Grouping(expr);
        }
    }


    bool EOF() {
        return tokens[i].type == TokenType.EOF;
    }
}