# frozen_string_literal: true

module Acme
  module Models
    # @see Acme::Resources::Widgets#create
    class Widget < Acme::Internal::Type::BaseModel
      # @!attribute id
      #
      #   @return [String]
      required :id, String

      # @!attribute created_at
      #
      #   @return [Time]
      required :created_at, Time

      # @!attribute name
      #
      #   @return [String]
      required :name, String

      # @!attribute status
      #   Where the widget is in its life cycle.
      #
      #   @return [Symbol, Acme::Models::Widget::Status]
      required :status, enum: -> { Acme::Widget::Status }

      # @!attribute homepage_url
      #
      #   @return [String, nil]
      optional :homepage_url, String

      # @!attribute labels
      #   Labels by language.
      #
      #   @return [Hash{Symbol=>Acme::Models::LabelsItem}, nil]
      optional :labels, -> { Acme::Internal::Type::HashOf[Acme::LabelsItem] }

      # @!attribute parts
      #
      #   @return [Array<Acme::Models::Widget::Part>, nil]
      optional :parts, -> { Acme::Internal::Type::ArrayOf[Acme::Widget::Part] }

      # @!attribute sku
      #
      #   @return [String, nil]
      optional :sku, String, nil?: true

      # @!attribute weight_kg
      #
      #   @return [Float, nil]
      optional :weight_kg, Float

      # @!method initialize(id:, created_at:, name:, status:, homepage_url: nil, labels: nil, parts: nil, sku: nil, weight_kg: nil)
      #   A widget in the catalogue.
      #
      #   @param id [String]
      #
      #   @param created_at [Time]
      #
      #   @param name [String]
      #
      #   @param status [Symbol, Acme::Models::Widget::Status] Where the widget is in its life cycle.
      #
      #   @param homepage_url [String]
      #
      #   @param labels [Hash{Symbol=>Acme::Models::LabelsItem}] Labels by language.
      #
      #   @param parts [Array<Acme::Models::Widget::Part>]
      #
      #   @param sku [String, nil]
      #
      #   @param weight_kg [Float]

      # Where the widget is in its life cycle.
      #
      # @see Acme::Models::Widget#status
      module Status
        extend Acme::Internal::Type::Enum

        DRAFT = :draft
        ACTIVE = :active
        RETIRED = :retired

        # @!method self.values
        #   @return [Array<Symbol>]
      end

      class Part < Acme::Internal::Type::BaseModel
        # @!attribute label
        #
        #   @return [String, nil]
        optional :label, String

        # @!attribute part_number
        #
        #   @return [Integer, nil]
        optional :part_number, Integer

        # @!method initialize(label: nil, part_number: nil)
        #   @param label [String]
        #   @param part_number [Integer]
      end
    end
  end
end
