// One pull request for Focus to read: eight changed files, Risks findings on lines of two of them, one finding on a line the
// diff does not hold, and findings whose words name other changed files by path and line. Invented, as every fixture here is.

import { build, checks, commits } from "./pr-review-pages";

const ISSUER = "app/services/refund_issuer.rb";
const PAYMENT = "app/models/payment.rb";

export const refunds = build({
  repository: "fernhill/tally",
  manifestId: "tally",
  number: 2417,
  title: "Refund against the original payment",
  branch: "odile/refund-original-payment",
  author: "odile",
  lists: [
    "requested",
  ],
  opened: 95,
  body: "## Problem\n\nA refund was a new charge in reverse, so it could reach the card after the payment it undid had been disputed.\n\n## Change\n\nA refund now goes back against the payment it came from, never more than is left of it. A gateway timeout leaves the refund pending and a job tries it again.\n\n## Testing\n\nRefund part of a payment from its page, then try to refund more than is left.",
  checks: "passed",
  review: "none",
  conversation: [],
  checkRows: checks("fernhill/tally", 2417, [
    [
      "rspec",
      "passed",
      310,
    ],
    [
      "rubocop",
      "passed",
      40,
    ],
  ]),
  commits: commits([
    [
      "5e1b0a4",
      "Refund against the payment it came from",
      "odile",
      120,
    ],
    [
      "9c27d13",
      "Try a refund again after a gateway timeout",
      "odile",
      100,
    ],
  ]),
  summary:
    "A refund was a new charge in reverse. It now goes back against the original payment, capped at what is left of it, and a timeout is tried again by a job.",
  explainedBy: "agent",
  themes: [
    {
      id: "original",
      title: "A refund goes back to the payment it came from",
      purpose: "The money returns the way it came, and never more than the payment had left.",
      changes: [
        {
          id: "issue",
          title: "Issue against the charge",
          why: "Refunding against the charge keeps the gateway's dispute and refund records on one payment.",
          hunks: [
            [
              ISSUER,
              [
                "@@ -1,10 +1,20 @@",
                " class RefundIssuer",
                "   def initialize(payment)",
                "     @payment = payment",
                "   end",
                " ",
                "   def issue(amount_cents)",
                "-    refund = @payment.refunds.create!(amount_cents: amount_cents)",
                "-    Gateway.refund(refund)",
                "+    # Refund against the payment it came from, never a new charge.",
                "+    raise Refund::TooLarge unless @payment.refundable?(amount_cents)",
                "+",
                '+    refund = @payment.refunds.create!(amount_cents: amount_cents, state: "pending")',
                "+    settle(refund)",
                "+  end",
                "+",
                "+  def settle(refund)",
                "+    Gateway.refund(refund, against: @payment.charge_id)",
                '+    refund.update!(state: "succeeded")',
                "+  rescue Gateway::Timeout",
                "+    RefundRetryJob.perform_later(refund.id)",
                "   end",
                " end",
              ],
            ],
          ],
        },
        {
          id: "refundable",
          title: "Cap a refund at what is left",
          why: "A payment refunded in parts must not give back more than it took.",
          hunks: [
            [
              PAYMENT,
              [
                "@@ -14,6 +14,14 @@ class Payment < ApplicationRecord",
                "   has_many :refunds",
                "   belongs_to :order",
                " ",
                "+  def refundable_amount",
                '+    amount_cents - refunds.where(state: "succeeded").sum(:amount_cents)',
                "+  end",
                "+",
                "+  def refundable?(amount_cents)",
                "+    amount_cents.positive? && amount_cents <= refundable_amount",
                "+  end",
                "+",
                "   def charge_id",
                "     gateway_reference",
                "   end",
              ],
            ],
          ],
        },
        {
          id: "endpoint",
          title: "Ask for a refund from the payment's page",
          why: "The form posts to the payment's own refunds, so the issuer is handed the right payment.",
          hunks: [
            [
              "app/controllers/refunds_controller.rb",
              [
                "@@ -8,6 +8,9 @@ class RefundsController < ApplicationController",
                "   def create",
                "     payment = Payment.find(params[:payment_id])",
                "-    payment.charge.refund!(refund_params[:amount_cents])",
                "+    RefundIssuer.new(payment).issue(refund_params[:amount_cents])",
                '+    redirect_to payment, notice: "Refund requested."',
                "+  rescue Refund::TooLarge",
                "+    render :new, status: :unprocessable_entity",
                "   end",
                " ",
                "   private",
              ],
            ],
            [
              "config/routes.rb",
              [
                "@@ -21,5 +21,5 @@ Rails.application.routes.draw do",
                "   resources :payments, only: %i[show] do",
                "-    resources :refunds, only: %i[index]",
                "+    resources :refunds, only: %i[index new create]",
                "   end",
                " ",
                "   resources :orders",
              ],
            ],
            [
              "app/views/refunds/_form.html.erb",
              [
                "@@ -0,0 +1,5 @@",
                "+<%= form_with url: payment_refunds_path(@payment) do |form| %>",
                '+  <%= form.label :amount_cents, "Amount" %>',
                "+  <%= form.number_field :amount_cents, max: @payment.refundable_amount %>",
                '+  <%= form.submit "Refund" %>',
                "+<% end %>",
              ],
            ],
          ],
        },
      ],
    },
    {
      id: "timeout",
      title: "A refund that times out is tried again",
      purpose: "A slow gateway leaves the refund pending rather than lost.",
      changes: [
        {
          id: "retry",
          title: "Retry from a job",
          why: "The request that asked for the refund should not wait on the gateway's retries.",
          hunks: [
            [
              "app/jobs/refund_retry_job.rb",
              [
                "@@ -0,0 +1,11 @@",
                "+class RefundRetryJob < ApplicationJob",
                "+  queue_as :refunds",
                "+  retry_on Gateway::Timeout, wait: :polynomially_longer, attempts: 5",
                "+",
                "+  def perform(refund_id)",
                "+    refund = Refund.find(refund_id)",
                "+    return unless refund.pending?",
                "+",
                "+    RefundIssuer.new(refund.payment).settle(refund)",
                "+  end",
                "+end",
              ],
            ],
            [
              "db/migrate/20261102_add_state_to_refunds.rb",
              [
                "@@ -0,0 +1,5 @@",
                "+class AddStateToRefunds < ActiveRecord::Migration[7.1]",
                "+  def change",
                '+    add_column :refunds, :state, :string, null: false, default: "pending"',
                "+  end",
                "+end",
              ],
            ],
          ],
        },
        {
          id: "spec",
          title: "Cover the cap and the charge",
          why: "The two promises the change makes, each with a test.",
          hunks: [
            [
              "spec/services/refund_issuer_spec.rb",
              [
                "@@ -0,0 +1,14 @@",
                "+RSpec.describe RefundIssuer do",
                "+  let(:payment) { create(:payment, amount_cents: 5_000) }",
                "+",
                '+  it "refunds against the original charge" do',
                "+    expect(Gateway).to receive(:refund).with(anything, against: payment.charge_id)",
                "+    described_class.new(payment).issue(2_000)",
                "+  end",
                "+",
                '+  it "refuses more than is left" do',
                '+    create(:refund, payment: payment, amount_cents: 4_000, state: "succeeded")',
                "+    expect { described_class.new(payment).issue(2_000) }",
                "+      .to raise_error(Refund::TooLarge)",
                "+  end",
                "+end",
              ],
            ],
          ],
        },
      ],
    },
  ],
  results: {
    risks: [
      {
        change: "issue",
        title: "Two refunds can pass the same check",
        body: "The cap is read, then the refund is written, with no lock between them. Two requests at once each see the whole amount left, because `app/models/payment.rb:17-19` counts only refunds that have settled.",
        severity: "high",
        match: "raise Refund::TooLarge",
        through: 'state: "pending"',
      },
      {
        change: "issue",
        title: "A timeout leaves the refund pending until the job runs",
        body: "The page says the refund was requested while it waits in the queue. `refund_retry_job.rb:9` settles it later, with no word to the person who asked.",
        severity: "low",
        match: "rescue Gateway::Timeout",
        through: "RefundRetryJob.perform_later",
      },
      {
        change: "refundable",
        title: "refundable_amount counts only succeeded refunds",
        body: "A pending refund is not subtracted, so the form's maximum and the cap both allow more than is left while one waits in `refund_retry_job.rb`.",
        severity: "high",
        match: "def refundable_amount",
        through: 'where(state: "succeeded")',
      },
      {
        change: "refundable",
        title: "Old payments have no currency",
        body: "Payments made before currencies were recorded hold none, and the cap compares cents without one.",
        severity: "medium",
        place: {
          path: PAYMENT,
          side: "right",
          start: 6,
          end: 6,
        },
      },
    ],
  },
});
