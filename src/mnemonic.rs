use std::sync::OnceLock;

/// High-level category for a ladder instruction mnemonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderMnemonicCategory {
    BcdArithmetic,
    BcdBinConversion,
    BinaryArithmetic,
    Exchange,
    BasicInstructions,
    LogicalOperations,
    DataTransfer,
    DataControl,
    DataProcessing,
    DataTableProcessing,
    DataTypeConversion,
    Loop,
    MotionControl,
    StringProcessing,
    SignInversion,
    Branching,
    Comparison,
    Time,
    System,
    WordBitControl,
    FAreaControl,
    Positioning,
    Shift,
    Interrupt,
    IncrementDecrement,
    TimerCounter,
    Communication,
    SpecialFunctions,
    SpecialCommunication,
    File,
    Display,
    Flag,
    Rotation,
}

impl LadderMnemonicCategory {
    /// User-facing English label for this category.
    pub fn label(self) -> &'static str {
        match self {
            Self::BcdArithmetic => "BCD Arithmetic",
            Self::BcdBinConversion => "BCD/BIN Conversion",
            Self::BinaryArithmetic => "Binary Arithmetic",
            Self::Exchange => "Exchange",
            Self::BasicInstructions => "Basic Instructions",
            Self::LogicalOperations => "Bitwise Operations",
            Self::DataTransfer => "Data Transfer",
            Self::DataControl => "Data Control",
            Self::DataProcessing => "Data Processing",
            Self::DataTableProcessing => "Data Table Processing",
            Self::DataTypeConversion => "Data Type Conversion",
            Self::Loop => "Loop",
            Self::MotionControl => "Motion Control",
            Self::StringProcessing => "String Processing",
            Self::SignInversion => "Sign Inversion",
            Self::Branching => "Branching",
            Self::Comparison => "Comparison",
            Self::Time => "Time",
            Self::System => "System",
            Self::WordBitControl => "Word Bit Control",
            Self::FAreaControl => "F Area Control",
            Self::Positioning => "Positioning",
            Self::Shift => "Shift",
            Self::Interrupt => "Interrupt",
            Self::IncrementDecrement => "Increment/Decrement",
            Self::TimerCounter => "Timer/Counter",
            Self::Communication => "Communication",
            Self::SpecialFunctions => "Special Functions",
            Self::SpecialCommunication => "Special/Communication",
            Self::File => "File",
            Self::Display => "Display",
            Self::Flag => "Flag",
            Self::Rotation => "Rotation",
        }
    }
}

/// Category and short description for a known ladder instruction mnemonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LadderMnemonicInfo {
    pub mnemonic: &'static str,
    pub category: LadderMnemonicCategory,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct LadderMnemonicGroup {
    category: LadderMnemonicCategory,
    mnemonics: &'static str,
    description: &'static str,
}

const fn group(
    category: LadderMnemonicCategory,
    mnemonics: &'static str,
    description: &'static str,
) -> LadderMnemonicGroup {
    LadderMnemonicGroup {
        category,
        mnemonics,
        description,
    }
}

static KNOWN_LADDER_MNEMONICS: OnceLock<Vec<LadderMnemonicInfo>> = OnceLock::new();

const LADDER_MNEMONIC_GROUPS: &[LadderMnemonicGroup] = &[
    group(
        LadderMnemonicCategory::BasicInstructions,
        "LOAD;LOAD NOT;LOADP;LOADN;LOADP NOT;LOADN NOT;AND;AND NOT;ANDP;ANDN;ANDP NOT;ANDN NOT;OR;OR NOT;ORP;ORN;ORP NOT;ORN NOT;R_EDGE;F_EDGE;AND LOAD;OR LOAD;MPUSH;MLOAD;MPOP;NOT;MCS;MCSCLR;OUT;OUT NOT;OUTP;OUTN;SET;RST;RESET;FF;END;NOP;BRST;BRSTP",
        "Basic ladder instruction.",
    ),
    group(
        LadderMnemonicCategory::TimerCounter,
        "TON;TOFF;TMR;TMON;TRTG;CTD;CTU;CTUD;CTR",
        "Timer/counter instruction.",
    ),
    group(
        LadderMnemonicCategory::DataTransfer,
        "MOV;MOVP;DMOV;DMOVP;MOV4;MOV4P;MOV8;MOV8P;CMOV;CMOVP;DCMOV;DCMOVP;GMOV;GMOVP;FMOV;FMOVP;BMOV;BMOVP;GBMOV;GBMOVP;RMOV;RMOVP;LMOV;LMOVP;$MOV;$MOVP",
        "Data transfer instruction.",
    ),
    group(
        LadderMnemonicCategory::BcdBinConversion,
        "BCD;BCDP;DBCD;DBCDP;BCD4;BCD4P;BCD8;BCD8P;BIN;BINP;DBIN;DBINP;BIN4;BIN4P;BIN8;BIN8P;GBCD;GBCDP;GBIN;GBINP;WTODW;WTODWP;DWTOW;DWTOWP",
        "BCD/BIN conversion instruction.",
    ),
    group(
        LadderMnemonicCategory::DataTypeConversion,
        "I2R;I2RP;I2L;I2LP;D2R;D2RP;D2L;D2LP;R2I;R2IP;R2D;R2DP;L2I;L2IP;L2D;L2DP;R2L;R2LP;L2R;L2RP;U2R;U2RP;U2L;U2LP;UD2R;UD2RP;UD2L;UD2LP;R2U;R2UP;R2UD;R2UDP;L2U;L2UP;L2UD;L2UDP",
        "Data type conversion instruction.",
    ),
    group(
        LadderMnemonicCategory::Comparison,
        "=;<>;>;<;>=;<=;=3;<>3;>3;<3;>=3;<=3;4=;4<>;4>;4<;4>=;4<=;8=;8<>;8>;8<;8>=;8<=;CMP;CMPP;DCMP;DCMPP;CMP4;CMP4P;CMP8;CMP8P;TCMP;TCMPP;DTCMP;DTCMPP;GEQ;GEQP;GGT;GGTP;GLT;GLTP;GGE;GGEP;GLE;GLEP;GNE;GNEP;GDEQ;GDEQP;GDGT;GDGTP;GDLT;GDLTP;GDGE;GDGEP;GDLE;GDLEP;GDNE;GDNEP;LOAD X;LOADD X;AND X;ANDD X;OR X;ORD X;LOADR X;LOADL X;ANDR X;ANDL X;ORR X;ORL X;LOAD$ X;AND$ X;OR$ X;LOADG X;LOADDG X;ANDG X;ANDDG X;ORG X;ORDG X;LOAD3 X;LOADD3 X;AND3 X;ANDD3 X;OR3 X;ORD3 X;LOAD4 X;LOAD8 X;AND4 X;AND8 X;OR4X;OR8X;ULOAD X;ULOADD X;UAND X;UANDD X;UOR X;UORD X",
        "Comparison instruction.",
    ),
    group(
        LadderMnemonicCategory::IncrementDecrement,
        "INC;INCP;DINC;DINCP;INC4;INC4P;INC8;INC8P;DEC;DECP;DDEC;DDECP;DEC4;DEC4P;DEC8;DEC8P;INCU;INCUP;DINCU;DINCUP;DECU;DECUP;DDECU;DDECUP",
        "Increment/decrement instruction.",
    ),
    group(
        LadderMnemonicCategory::Rotation,
        "ROL;ROLP;DROL;DROLP;ROL4;ROL4P;ROL8;ROL8P;ROR;RORP;DROR;DRORP;ROR4;ROR4P;ROR8;ROR8P;RCL;RCLP;DRCL;DRCLP;RCL4;RCL4P;RCL8;RCL8P;RCR;RCRP;DRCR;DRCRP;RCR4;RCR4P;RCR8;RCR8P",
        "Rotation instruction.",
    ),
    group(
        LadderMnemonicCategory::Shift,
        "BSFT;BSFTP;BSFL;BSFLP;DBSFL;DBSFLP;BSFL4;BSFL4P;BSFL8;BSFL8P;BSFR;BSFRP;DBSFR;DBSFRP;BSFR4;BSFR4P;BSFR8;BSFR8P;WSFT;WSFTP;WSFL;WSFLP;WSFR;WSFRP;SR;BRR;BRRP;BRL;BRLP",
        "Move/shift instruction.",
    ),
    group(
        LadderMnemonicCategory::Exchange,
        "XCHG;XCHGP;DXCHG;DXCHGP;GXCHG;GXCHGP;SWAP;SWAPP;GSWAP;GSWAPP;SWAP2;SWAP2P;GSWAP2;GSWAP2P",
        "Exchange instruction.",
    ),
    group(
        LadderMnemonicCategory::BinaryArithmetic,
        "$ADD;$ADDP;ADD;ADDP;DADD;DADDP;SUB;SUBP;DSUB;DSUBP;MUL;MULP;DMUL;DMULP;DIV;DIVP;DDIV;DDIVP;ADDU;ADDUP;DADDU;DADDUP;SUBU;SUBUP;DSUBU;DSUBUP;MULU;MULUP;DMULU;DMULUP;DIVU;DIVUP;DDIVU;DDIVUP;RADD;RADDP;LADD;LADDP;RSUB;RSUBP;LSUB;LSUBP;RMUL;RMULP;LMUL;LMULP;RDIV;RDIVP;LDIV;LDIVP;GADD;GADDP;GSUB;GSUBP",
        "Binary arithmetic instruction.",
    ),
    group(
        LadderMnemonicCategory::BcdArithmetic,
        "ADDB;ADDBP;ADDCP;DADDB;DADDBP;SUBB;SUBBP;DSUBB;DSUBBP;MULB;MULBP;DMULB;DMULBP;DIVB;DIVBP;DDIVB;DDIVBP",
        "BCD arithmetic instruction.",
    ),
    group(
        LadderMnemonicCategory::LogicalOperations,
        "WAND;WANDP;DWAND;DWANDP;WOR;WORP;DWOR;DWORP;WXOR;WXORP;DWXOR;DWXORP;WXNR;WXNRP;DWXNR;DWXNRP;GWAND;GWANDP;GWOR;GWORP;GWXOR;GWXORP;GWXNR;GWXNRP;BAND;BANDP;BOR;BORP;BXOR;BXORP;BXNR;BXNRP;ABAND;ABANDP;ABOR;ABORP;ABXOR;ABXORP;ABXNR;ABXNRP",
        "Bitwise operation instruction.",
    ),
    group(
        LadderMnemonicCategory::Display,
        "SEG;SEGP",
        "Display instruction.",
    ),
    group(
        LadderMnemonicCategory::DataProcessing,
        "BSUM;BSUMP;DBSUM;DBSUMP;BRST;BRSTP;ENCO;ENCOP;DECO;DECOP;DIS;DISP;UNI;UNIP;WTOB;WTOBP;BETOW;BTOWP;IORF;IORFP;SCH;SCHP;DSCH;DSCHP;MAX;MAXP;DMAX;DMAXP;MIN;MINP;DMIN;DMINP;SUM;SUMP;DSUM;DSUMP;AVE;AVEP;DAVE;DAVEP;MUX;MUXP;DMUX;DMUXP;DETECT;DETECTP;RAMP;SORT;DSORT;TRAMP;RTRAMP;ADS;ADSP;ADU;ADUP;INLATCH",
        "Data processing instruction.",
    ),
    group(
        LadderMnemonicCategory::DataTableProcessing,
        "FIWR;FIWRP;FIFRD;FIFRDP;FILRD;FILRDP;FIINS;FIINSP;FIDEL;FIDELP",
        "Data table processing instruction.",
    ),
    group(
        LadderMnemonicCategory::StringProcessing,
        "BINDA;BINDAP;DBINDA;DBINDAP;BINHA;BINHAP;DBINHA;DBINHAP;BCDDA;BCDDAP;DBCDDA;DBCDDAP;DABIN;DABINP;DDABIN;DDABINP;HABIN;HABINP;DHABIN;DHABINP;DABCD;DABCDP;DDABCD;DDABCDP;LEN;LENP;STR;STRP;VAL;VALP;DVAL;DVALP;RSTR;RSTRP;LSTR;LSTRP;STRR;STRRP;STRL;STRLP;ASC;ASCP;HEX;HEXP;RIGHT;RIGHTP;LEFT;LEFTP;MID;MIDP;REPLACE;REPLACEP;FIND;FINDP;RBCD;RBCDP;LBCD;LBCDP;BCDR;BCDRP;BCDL;BCDLP;GFIND",
        "String processing instruction.",
    ),
    group(
        LadderMnemonicCategory::SpecialFunctions,
        "SIN;SINP;ASIN;ASINP;COS;COSP;ACOS;ACOSP;TAN;TANP;ATAN;ATANP;RAD;RADP;DEG;DEGP;SQRT;SQRTP;BSQRT;BSQRTP;BDSQRT;BDSQRTP;LN;LNP;LOG;LOGP;EXP;EXPP;EXPT;EXPTP",
        "Special function instruction.",
    ),
    group(
        LadderMnemonicCategory::DataControl,
        "LIMIT;LIMITP;DLIMIT;DLIMITP;DZONE;DZONEP;DDZONE;DDZONEP;DZONES;DZONESP;DDZONES;DDZONESP;VZONE;VZONEP;DVZONE;DVZONEP;PIDRUN;PIDPRMT;PIDPAUSE;PIDINIT;PIDAT;PIDHBD;PIDCAS;SCAL;SCALP;DSCAL;DSCALP;RSCAL;RSCALP;SCAL2;SCAL2P;DSCAL2;DSCAL2P;RSCAL2;RSCAL2P",
        "Data control instruction.",
    ),
    group(
        LadderMnemonicCategory::Time,
        "DATERD;DATERDP;DATERD2;DATERD2P;DATEWR;DATEWRP;ADDCLK;ADDCLKP;SUBCLK;SUBCLKP;SECOND;SECONDP;HOUR;HOURP;ADDCAL;SUBCAL",
        "Time-related instruction.",
    ),
    group(
        LadderMnemonicCategory::Branching,
        "JMP;LABEL;CALL;CALLP;SBRT;RET",
        "Branching instruction.",
    ),
    group(
        LadderMnemonicCategory::Loop,
        "FOR;NEXT;BREAK",
        "Loop instruction.",
    ),
    group(
        LadderMnemonicCategory::Flag,
        "STC;CLC;CLE",
        "Flag instruction.",
    ),
    group(
        LadderMnemonicCategory::System,
        "FALS;DUTY;TFLK;WDT;WDTP;OUTOFF;STOP;ESTOP;INIT_DONE",
        "System instruction.",
    ),
    group(
        LadderMnemonicCategory::Interrupt,
        "EI;DI;EIN;DIN",
        "Interrupt instruction.",
    ),
    group(
        LadderMnemonicCategory::SignInversion,
        "NEG;NEGP;DNEG;DNEGP;RNEG;RNEGP;LNEG;LNEGP;ABS;ABSP;DABS;DABSP",
        "Sign inversion instruction.",
    ),
    group(
        LadderMnemonicCategory::File,
        "RSET;RSETP;EMOV;EMOVP;EDMOV;EDMOVP;EBREAD;EBWRITE;EBCMP;EERRST",
        "File instruction.",
    ),
    group(
        LadderMnemonicCategory::FAreaControl,
        "FSET;FRST;FWRITE",
        "F area control instruction.",
    ),
    group(
        LadderMnemonicCategory::WordBitControl,
        "LOADB;LOADBN;ANDB;ANDBN;ORB;ORBN;BOUT;BSET;BRESET",
        "Word bit control instruction.",
    ),
    group(
        LadderMnemonicCategory::SpecialCommunication,
        "GET;GETP;GETE;GETEP;PUT;PUTP",
        "Special/communication-related instruction.",
    ),
    group(
        LadderMnemonicCategory::Communication,
        "P2PSN;P2PWRD;P2PWWR;P2PBRD;P2PBWR;SNDUDATA;RCVUDATA;SENDDTR;SENDRTS;GETIP;SETIP;MNETINFO;MSETIP;LNETINFO;MGETLED;GETCOMM;PUTCOMM;FCS;SETPORT;GETPORT;SETPARAM;GETPARAM;CPMSG;MSETDNETS",
        "Communication instruction.",
    ),
    group(
        LadderMnemonicCategory::Positioning,
        "ORG;FLT;DST;IST;LIN;CIN;SST;VTP;PTV;STP;SKP;SSP;SSS;POR;SOR;PSO;NMV;INCH;RTP;SNS;SRS;MOF;PRS;ZOE;ZOD;EPRS;TEA;TEAA;EMG;CLR;ECLR;PST;TBP;TEP;THP;TMP;TSP;TCP;TMD;WRT;SRD;PWR;TWR;VRD;VWR;RCP;VTPP;PWM;XORG;XFLT;XDST;XIST;XSST;XVTP;XVTPP;XPTV;XPTT;XSTP;XSKP;XSSP;XSSS;XPOR;XSOR;XPSO;XNMV;XINCH;XRTP;XSNS;XSRS;XMOF;XPRS;XEPRS;XTEAA;XEMG;XCLR;XECLR;XPST;XSBP;XSEP;XSHP;XSMP;XSES;XSCP;XSMD;XWRT;XSRD;XCAM;XELIN;XSSSP;XPWR;XTWR;XSWR;XVRD;XVWR;XECON;XDCON;XSVON;XSVOFF;XSCLR;XSECLR;XCAMO;XRSTR;XSVPRD;XSVPWR;XSVSAVE;XTRQ;XLRD;XLCLR;XLSET;XSTC;XPHASING;XSSSD;XSSSPD;XSETOVR;XCAMA;XTPROBE;XABORTT;XTRQSL;XGEARIP;XCCCONEX;XCCCONON;XPLOOPONEX;XPLOOPON;XCCCOFFEX;XCCCOFF;XPLOOPOFFEX;XPLOOPOFF;XORGMEX;XORGM",
        "Positioning instruction.",
    ),
    group(
        LadderMnemonicCategory::MotionControl,
        "GETM;GETMP;PUTM;PUTMP;XTURN;XGET;XGETP;XPUT;XPUTP",
        "Motion control instruction.",
    ),
];

/// Return metadata for every known ladder mnemonic.
pub fn known_ladder_mnemonics() -> &'static [LadderMnemonicInfo] {
    KNOWN_LADDER_MNEMONICS
        .get_or_init(build_known_ladder_mnemonics)
        .as_slice()
}

/// Return category and description metadata for a ladder mnemonic.
pub fn ladder_mnemonic_info(mnemonic: &str) -> Option<LadderMnemonicInfo> {
    known_ladder_mnemonics()
        .iter()
        .copied()
        .find(|info| info.mnemonic == mnemonic)
}

fn build_known_ladder_mnemonics() -> Vec<LadderMnemonicInfo> {
    let mut mnemonics: Vec<LadderMnemonicInfo> = Vec::new();

    for group in LADDER_MNEMONIC_GROUPS {
        for mnemonic in group.mnemonics.split(';').map(str::trim) {
            if mnemonic.is_empty() || mnemonics.iter().any(|info| info.mnemonic == mnemonic) {
                continue;
            }

            mnemonics.push(LadderMnemonicInfo {
                mnemonic,
                category: group.category,
                description: group.description,
            });
        }
    }

    mnemonics
}
