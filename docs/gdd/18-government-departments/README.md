---
title: Government Departments & Bureaucracy
section: 18
status: design-complete
depends_on: [01, 06, 07, 12]
blocks: [19]
---

# Government Departments & Bureaucracy

## Design Principle

Every government entity — from the DMV to the CIA — is a playable/interactable workplace with real structure, real data, and real gameplay dynamics.

## Government Organization Entity

```
Government Org Entity (ECS)
├─ name, acronym, parent_department
├─ type:      cabinet_dept | independent_agency | regulatory_commission |
│             intelligence | law_enforcement | military_branch |
│             state_agency | local_dept
├─ mission    what this org does
├─ budget     annual, source
├─ headcount  total employees by grade/rank
├─ leadership appointed (political) vs. career (civil service)
├─ structure  divisions[], offices[], field_offices[]
├─ jurisdiction what laws/areas this org oversees
├─ politics   internal factions, turf wars, culture
├─ public_opinion approval rating
└─ morale     0-100 (affects org competence)
```

## Federal Government Hierarchy

### Executive Office of the President
White House Staff, OMB, NSC, CEA, OSTP

### Cabinet Departments (15)
State, Treasury, Defense (Army/Navy/AF/Marines/Space Force, Joint Chiefs, NSA, DIA), Justice (FBI, DEA, ATF, Marshals), Interior, Agriculture, Commerce, Labor, HHS, HUD, Transportation, Energy, Education, Veterans Affairs, Homeland Security (FEMA, Secret Service, ICE, CBP, TSA, Coast Guard)

### Independent Agencies
CIA, EPA, NASA, SBA, SSA, USPS, GSA

### Regulatory Commissions
FCC, SEC, FTC, FEC, NRC, NLRB, Federal Reserve

### State Level (generated per state)
Governor's office, State AG, Secretary of State, state police, DMV, education board, health department

### Local Level (generated per district)
Police, fire, public works, parks & rec, building/zoning, schools, county clerk

## Career Paths Within Government

### Civil Service Track
- GS-5 to GS-15 (real federal pay scale)
- Promotion: performance + time-in-grade + office politics
- Senior Executive Service (SES): top career tier
- Unique tension: career staff serve regardless of who wins White House

### Political Appointee Track
- ~4,000 positions in federal government
- Appointed by president, some require Senate confirmation
- Serve at pleasure of president
- Cabinet Secretary: run entire department, attend NSC

### Law Enforcement Track
- Beat cop → detective → lieutenant → chief
- FBI agent → supervisor → SAC → deputy director → director
- Federal prosecutor → US Attorney → AG
- Can pivot to elected office (DA → AG → Governor)

### Military Track
- Enlisted or officer path, O-1 through O-10
- Joint Chiefs appointment (presidential nomination)
- Can pivot to elected office (Eisenhower path)

### Intelligence Track
- CIA analyst → station chief → division head → director
- Uniquely isolated — can't discuss work with most NPCs

### Judiciary Track
- Law clerk → practice → federal judge nomination
- District → Circuit → Supreme Court
- Lifetime appointment. Rulings CHANGE how the law engine works.

## Department-Specific Gameplay

### How It Differs from Elected Office

| Elected Official | Department Career |
|-----------------|-------------------|
| Public facing | Behind the scenes |
| Approval matters | Performance reviews matter |
| Campaign fundraising | Budget justification |
| Media presence required | Media exposure = usually bad |
| Broad issue portfolio | Deep domain expertise |
| Term-limited | Career-length (decades) |

### Unique Mechanics

**Budget Wars**: Department budget set by Congress. Lobby for your slice. Rival departments compete. OMB reviews. Budget cuts = reduced capability = lower morale = worse outcomes.

**Regulatory Capture**: Industries you regulate try to co-opt you. Lobbyists offer private sector jobs. Industry-friendly positions = easier job but scandal risk. Aggressive regulation = industry opposition.

**Whistleblower Dynamics**: Discover wrongdoing. Report internally (may be buried), to IG (slower but safer), to Congress (protection but career risk), to press (max impact, max risk). Or cover it up.

**Interagency Politics**: CIA vs FBI turf wars, DoD vs State on foreign policy, EPA vs Commerce on regulations. Social graph spans departments.

**Presidential Transition**: New president = new appointees above you. Your programs may be killed or expanded. Career staff: survive but adapt. Appointees: fired. Resist quietly, comply, resign in protest, or sabotage.

## Data Sources

| Source | Data |
|--------|------|
| USAspending.gov | Budget by agency, program, year |
| OPM FedScope | Federal workforce (headcount by agency, grade, location) |
| Federal Register | Regulations published by each agency |
| GAO Reports | Audits and investigations |
| GS Pay Scale | Real salary tables by grade and locality |

## Scenario Files

```
game/scenarios/modern_usa/government/
├─ federal_departments.toml
├─ independent_agencies.toml
├─ regulatory_commissions.toml
├─ career_paths/
│  ├─ civil_service.toml
│  ├─ law_enforcement.toml
│  ├─ military.toml
│  ├─ intelligence.toml
│  └─ judiciary.toml
└─ department_dynamics/
   ├─ budget_process.rhai
   ├─ turf_wars.rhai
   └─ transition.rhai
```
